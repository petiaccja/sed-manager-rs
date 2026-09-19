# Privilege separation plan

This is a rough plan or idea about how to strip away privileges from the GUI
and OTLP, which pull in half of `crates.io` and risk supply chain attacks. The
plan is not final, but it gives a general idea about how it may be done.

## Why

The GUI process currently runs with elevated privileges (root/Administrator)
because it needs to issue raw SCSI/NVMe security-protocol commands to the
drive. Everything else it links against (Slint + rendering/font/windowing
stack, and notably the full OTLP telemetry stack: `tonic` +
`opentelemetry-otlp` + `tracing-opentelemetry`, pulling in `hyper`/`h2`/a TLS
stack/`tower`/`prost`) has no legitimate need to run elevated, and inherits
that privilege for free today. The goal is to shrink what actually runs
elevated down to `sed_tper`/`sed_device`/`sed_packet`/`sed_spec` plus a thin
RPC layer, and move the GUI + telemetry export to an unprivileged process.

This is a pure hardening change with **no functional improvement**. It's
justified specifically because of concrete, non-hypothetical risk (the OTLP
stack alone is a large dependency graph with zero reason to run as root), not
because of a generic "least privilege" instinct.

### Decisions already made, and why

- **A persistent privileged daemon is required, not a per-operation/per-session
  helper process.** Two independent reasons rule out spawning a fresh
  subprocess per call or per session:
    1. `LockingConfigSession` holds an open `Session` for a long time (as long
       as the user is configuring locking ranges), so the privileged side must
       stay alive across many calls.
    2. `Tper` must not be connected twice on the same ComID. A `SetupSession`
       and a `LockingConfigSession` for the _same device_ need to share one
       `Tper`/`Controller`. That sharing only works in-process — it can't cross
       an OS process boundary — so the smallest viable privileged unit is "one
       long-lived process per open device," which is no simpler than a shared
       daemon serving all devices.
       A stdio/CLI-style helper (`git`-style: spawn, pass args, read stdout) was
       considered and rejected: the data exchanged (structured backend types,
       bidirectional, many calls per session) doesn't fit a single-shot
       request/response shape, and switching transport from a socket to a pipe
       doesn't remove any of the framing/serialization/service-trait work below —
       it only would have removed socket-path/ACL management, which is a small
       fraction of the total design.
- **`tarpc` is worth its `tokio` dependency.** It lets backend types cross the
  RPC boundary directly via `serde`, avoiding a second manual
  backend-to-wire-type conversion pass (we already have one conversion pass
  to Slint types; a schema-based framework like `capnp-rpc` would have forced
  a third representation and a second conversion pass). `tarpc` generates the
  client/server from trait definitions, is transport-agnostic over anything
  `AsyncRead + AsyncWrite` (Unix sockets, named pipes, in-process channels),
  and its `Context` carries trace correlation that composes with the existing
  `tracing-opentelemetry` setup.
- **Services are split by object, not one state-checking monolith.** Each
  stateful local type (`SetupSession`, `LockingConfigSession`, a device
  connection) becomes its own service/trait. This makes invalid call
  sequences inexpressible (there is no method to call) instead of something
  checked at runtime.
- **A capability/factory pattern (`Endpoint<T>`) replaces a device-ID-keyed
  registry.** A factory method returns `Endpoint<T>` — a serializable
  URI-like handle with a `.connect()` — instead of an opaque ID looked up in
  a shared table on every call. This avoids a global lock touched by every
  RPC call, and gives each session/device its own connection with its own
  lifetime.
- **No device-open deduplication.** Preventing two `Tper`s on the same ComID
  _within this process_ doesn't prevent it from another process entirely
  (opening a device file isn't exclusive), so a dedup registry only solves a
  narrow slice of an unsolvable problem. Not worth building.
- **No open-sessions registry inside device state.** `Tper::close()` returns
  a `JoinHandle` that only resolves once every `Controller` clone (held by
  any live `Session` or `SessionStarter`) is dropped. Awaiting it already
  gives correct "wait for everything to finish" semantics for free. Because
  this is local IPC (Unix socket/named pipe), a crashed client's file
  descriptors are closed by the OS, so this wait is reliable in practice, not
  open-ended. The consequence: the GUI is responsible for closing/dropping
  its own session connections before/when closing a device, same discipline
  the local code already has today (`Session::close()` in
  `sed_manager_gui/src/session.rs`).
- **Keep the privileged binary's dependency graph minimal.** No GUI, no
  `tonic`/OTLP stack in the daemon — export telemetry from the unprivileged
  GUI process instead, or have the daemon emit local structured logs only.
  This is where most of the actual security payoff lives.
- **Supply-chain hardening (`cargo audit`/`cargo deny`, pinned and reviewed
  dependency bumps) is complementary, not a substitute.** It reduces the
  probability a dependency is compromised; privilege separation reduces the
  blast radius if one is anyway. Do both; the first is cheap and independent
  of this whole plan.

### Pending decisions

- **Consider an installed system service over per-launch elevation**. Once
  process splitting happens (systemd unit / Windows service, elevated once at
  install time) — avoids a UAC/pkexec prompt on every GUI launch or every
  device open, matching how VPN clients / Docker Desktop / printer services
  are conventionally built. However, services require a separate installation
  procedure as opposed to just distributing plain binaries, and they are a
  total pain in the ass to work with, at least on Windows.

## Target shape (end state, after all stages)

Four service traits, layered so each level's methods only need what's bound
to that connection — no ID passed on every call.

```rust
// Layer 0 — always on, one instance for the whole daemon.
#[tarpc::service]
pub trait Host {
    async fn list_devices() -> Result<Vec<DeviceInfo>, Error>;
    async fn open_device(device: DeviceId) -> Result<Endpoint<DeviceApiClient>, Error>;
}

// Layer 1 — one instance per opened device.
#[tarpc::service]
pub trait DeviceApi {
    async fn spec() -> Spec;
    async fn create_setup_session() -> Result<Endpoint<SetupSessionApiClient>, Error>;
    async fn create_locking_config_session(
        authority: AuthorityRef, password: Option<MaxBytes<32>>,
    ) -> Result<Endpoint<LockingConfigSessionApiClient>, Error>;
    async fn close() -> Result<(), Error>;
}

// Layer 2a — one instance per setup-session connection. Stateless w.r.t.
// the drive: SetupSession's own drive-side sessions are already
// short-lived/self-closing per call, so there's no close() here.
#[tarpc::service]
pub trait SetupSessionApi {
    async fn take_ownership(new_sid_password: MaxBytes<32>) -> Result<(), Error>;
    async fn activate_secondary_sp(sid_password: MaxBytes<32>) -> Result<(), Error>;
    async fn revert_tper(authority: AuthorityRef, password: MaxBytes<32>) -> Result<(), Error>;
    async fn revert_secondary_sp(sid_password: MaxBytes<32>) -> Result<(), Error>;
    async fn revert_secondary_sp_ex(
        admin: AuthorityRef, password: MaxBytes<32>, keep_global_range_key: Option<bool>,
    ) -> Result<(), Error>;
    async fn change_password(
        sp: SecurityProviderRef, authority: AuthorityRef, current_password: MaxBytes<32>, new_password: MaxBytes<32>,
    ) -> Result<(), Error>;
    async fn list_authorities(sp: SecurityProviderRef) -> Result<Vec<Authority>, Error>;
}

// Layer 2b — one instance per logged-in locking-config session.
#[tarpc::service]
pub trait LockingConfigSessionApi {
    async fn get_authorities() -> Result<Vec<Authority>, Error>;
    async fn get_locking_ranges() -> Result<Vec<LockingRange>, Error>;
    async fn get_mbr() -> Result<MbrControl, Error>;
    async fn close() -> Result<(), Error>;
}
```

### Server-side structures

```rust
// sed_tper additions: let a session-opening capability outlive/detach from
// the Tper struct itself, without needing Arc<Tper>.
pub trait SessionSource {
    async fn start_session(
        &self, sp: SecurityProviderRef, authority: Option<AuthorityRef>, password: Option<MaxBytes<32>>,
    ) -> Result<Session, Error>;
}
impl SessionSource for Tper { /* existing start_session body, unchanged */ }

#[derive(Clone)]
pub struct SessionStarter {
    controller: Controller,          // already Clone, Arc/channel-backed
    host_session_id: Arc<AtomicU32>, // Tper's field changes from AtomicU32 to this
}
impl SessionSource for SessionStarter { /* same body as Tper's, via the shared counter */ }
impl Tper {
    pub fn session_starter(&self) -> SessionStarter { /* clone controller + counter */ }
}

// SetupSession's methods generalize from `tper: &Tper` to `source: &impl SessionSource`.
// Backward compatible: existing callers passing &Tper still type-check.
```

```rust
struct DeviceState {
    tper: Mutex<Option<Tper>>,
    spec: Spec,
}
#[derive(Clone)]
struct DeviceServer(Arc<DeviceState>);

impl DeviceApi for DeviceServer {
    async fn create_setup_session(self, _: context::Context) -> Result<Endpoint<SetupSessionApiClient>, Error> {
        let tper = self.0.tper.lock().await;
        let tper = tper.as_ref().ok_or(Error::DeviceClosed)?;
        let server = SetupSessionServer { session: Arc::new(SetupSession::new(self.0.spec.clone())), starter: tper.session_starter() };
        spawn_single_shot_listener(server).await
    }

    async fn create_locking_config_session(
        self, _: context::Context, authority: AuthorityRef, password: Option<MaxBytes<32>>,
    ) -> Result<Endpoint<LockingConfigSessionApiClient>, Error> {
        let tper = self.0.tper.lock().await;
        let tper = tper.as_ref().ok_or(Error::DeviceClosed)?;
        let session = LockingConfigSession::login(tper, self.0.spec.clone(), authority, password).await?;
        // LockingConfigSession never keeps a Tper reference after login, only its own Session.
        spawn_single_shot_listener(LockingConfigSessionServer::new(session)).await
    }

    async fn close(self, _: context::Context) -> Result<(), Error> {
        if let Some(tper) = self.0.tper.lock().await.take() {
            tper.close().await.map_err(|_| Error::ShutdownFailed)?; // waits for all live sessions/starters to drop
        }
        Ok(())
    }
}
```

```rust
#[derive(Clone)]
struct SetupSessionServer {
    session: Arc<SetupSession>,
    starter: SessionStarter, // not Arc<Tper> — independent of the device connection's lifetime
}
impl SetupSessionApi for SetupSessionServer {
    async fn take_ownership(self, _: context::Context, new_sid_password: MaxBytes<32>) -> Result<(), Error> {
        self.session.take_owneship(&self.starter, new_sid_password).await
    }
    // every other method: same one-line shape
}
```

```rust
struct LockingConfigSessionInner {
    session: Mutex<Option<LockingConfigSession>>,
    cancel: Mutex<Option<CancelSender>>, // sed_async::cancel_channel()
}
#[derive(Clone)]
struct LockingConfigSessionServer(Arc<LockingConfigSessionInner>);

impl LockingConfigSessionApi for LockingConfigSessionServer {
    async fn close(self, _: context::Context) -> Result<(), Error> {
        if let Some(session) = self.0.session.lock().await.take() {
            session.close().await?; // real protocol close, not just drop
        }
        if let Some(cancel) = self.0.cancel.lock().await.take() {
            cancel.cancel(); // end the serve loop immediately instead of waiting for disconnect
        }
        Ok(())
    }
    // get_authorities/get_locking_ranges/get_mbr: error with Error::SessionClosed if the Option is None
}
// The spawned listener task also closes the session after its serve loop ends for ANY
// reason (explicit close, disconnect, crash) — idempotent safety net, since close()
// already leaves the Option empty in the graceful case.
```

```rust
// Generic capability handle, reused for every factory method above.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound = "")] // T never appears on the wire, don't require T: Serialize
pub struct Endpoint<T> { uri: String, _marker: PhantomData<T> }

pub trait RpcClient { fn from_transport(transport: impl Transport<...> + Send + 'static) -> Self; }
// one small hand-written impl per generated *Client type

impl<T: RpcClient> Endpoint<T> {
    pub async fn connect(&self) -> io::Result<T> { /* dispatch on uri scheme: unix://, tcp://, ... */ }
}
```

Each factory method (`open_device`, `create_setup_session`,
`create_locking_config_session`) spawns a listener that **accepts exactly
one connection** (bounded by a timeout, e.g. 30s) and then tears itself
down — bounds resource use without needing proactive cleanup elsewhere.

A `Weak`-referenced list at the `Host` level (not a lookup table — pure
enumeration) lets a shutdown sweep (SIGTERM/SIGINT, or service stop) close
every still-open device on process exit, mirroring the existing `#51`
device-close-on-quit fix.

## Staged implementation

### Stage 1 — reshape `sed_manager`/`sed_tper` in-process (current focus)

No RPC, no serialization, no process boundary yet. Just make the plain Rust
types already look like the future services, so later stages are mechanical.

1. `sed_tper`: add `SessionSource` trait + `SessionStarter` struct +
   `Tper::session_starter()`; change `Tper::host_session_id` from `AtomicU32`
   to `Arc<AtomicU32>`; generalize `SetupSession`'s methods from `&Tper` to
   `&impl SessionSource`.
2. `sed_manager`: add a `Device` type (the in-process analogue of
   `DeviceServer`/`DeviceState`) owning `Arc<dyn sed_device::Device>` +
   `Tper` + `Spec`, with methods mirroring `DeviceApi`:
   `create_setup_session(&self) -> SetupSession`,
   `create_locking_config_session(&self, authority, password) -> Result<LockingConfigSession, Error>`,
   `close(self) -> Result<(), Error>`.
3. `sed_manager`: add a `Host`-equivalent (name TBD, e.g. `DeviceManager`)
   with `list_devices()` and `open_device(path) -> Result<Device, Error>`,
   doing the open/discover/connect/discover-spec pipeline currently
   duplicated ad hoc in `sed_manager_gui`'s `DeviceList`/`Device`
   (`sed_manager_gui/sed_manager_gui/src/device_list.rs`).
4. Move `sed_manager_gui`'s device/session bookkeeping
   (`device_list.rs`, `session.rs`) to build on top of the new
   `sed_manager` types instead of duplicating device-open/Tper-connect
   logic in the GUI crate. Call sites in the GUI should already read like
   they're calling a remote service, just resolved locally.
5. Leave out for this stage (deferred to Stage 3/4, not needed yet):
   `Endpoint<T>`/`RpcClient`, cancellation tokens, `Serialize`/`Deserialize`
   on wire types, the single-accept-listener spawn helper, the `Weak`
   shutdown-sweep list. `LockingConfigSession::close()` already exists
   locally and is fine as-is for now.
6. Update/extend tests (virtual device infra in `sed_virtual_device`
   already supports this kind of exercise).

### Stage 2 — explicit async traits mirroring the service definitions (still in-process)

Define `HostApi`/`DeviceApi`/`SetupSessionApi`/`LockingConfigSessionApi` as
plain async Rust traits (no `tarpc` yet), matching the method signatures
sketched above minus the `Endpoint<T>` wrapping (factory methods just return
the owned object directly). Implement them for the Stage 1 structs. This
pins down exactly what's a wire parameter vs. server-side state before any
serialization is involved.

### Stage 3 — convert to real `tarpc` services over an in-process channel

Add `#[tarpc::service]` versions of the Stage 2 traits and serve them over an
in-process/duplex transport (still one binary, no real IPC). This is where:

- `Serialize`/`Deserialize` gets added to wire types: `MaxBytes` via
  `smallvec`'s `serde` feature (free), `sed_spec` objects via added derives
  alongside their existing ones, `sed_manager::Error` via a flattened
  serializable variant (its one non-mechanical piece).
- The `Endpoint<T>`/`RpcClient` factory pattern gets built and validated.
- Close/cancellation logic (`sed_async::cancel_channel`, the safety-net
  cleanup) gets wired up end-to-end.

Validates the whole RPC layer without needing sockets, ACLs, or elevation.

### Stage 4 — real process separation

- Split into two binaries: a minimal-dependency privileged daemon (no GUI,
  no `tonic`/OTLP stack) and the existing GUI as the unprivileged client.
- Swap the in-process transport for Unix domain sockets (Linux) / named
  pipes (Windows); `Endpoint<T>` URIs become real `unix://`/`tcp://`-style
  addresses; the single-accept-listener logic goes live for real.
- Decide and implement the deployment/elevation model — installed system
  service (systemd unit / Windows service) preferred over per-launch
  elevation.
- Wire the `Host`-level shutdown sweep to SIGTERM/SIGINT or service-stop.

#### Linux: what privilege is actually needed (superuser is not required)

Experiment (not 100% verified against every kernel version, but consistent
with known Linux capability gating): setting `CAP_SYS_ADMIN`,
`CAP_SYS_RAWIO`, and `CAP_DAC_OVERRIDE` as file capabilities on the
executable, run as a regular user, is sufficient — no `root`/`setuid`
needed. Breakdown per capability:

- **`CAP_SYS_RAWIO` — the ATA/SCSI passthrough path.** TCG commands to
  SATA/SAS drives (`TRUSTED SEND`/`RECEIVE` on ATA, `SECURITY PROTOCOL
  IN`/`OUT` on SCSI) go through `SG_IO`/`HDIO_DRIVE_CMD`-style passthrough
  ioctls. The kernel gates any command not on its small "known safe,
  read-only" whitelist behind `capable(CAP_SYS_RAWIO)` — there's no
  finer-grained capability for "security-protocol opcodes only." This is a
  property of the kernel's passthrough interface, not something fixable by
  changing this app's code.
- **`CAP_SYS_ADMIN` — almost certainly the NVMe admin-passthrough path.**
  NVMe Security Send/Receive (opcodes 0x81/0x82) go through the NVMe Admin
  command set, i.e. the `NVME_IOCTL_ADMIN_CMD` ioctl, which the kernel gates
  behind `capable(CAP_SYS_ADMIN)` unconditionally — it doesn't distinguish
  "Security Send" from "Format NVM" or "Firmware Commit." This is the
  broadest and most dangerous of the three: `CAP_SYS_ADMIN` is a well-known
  near-root grab-bag (mount/umount, namespaces, `bpf`, `perf_event_open`,
  keyrings, and many unrelated ioctls all ended up bucketed into it over the
  kernel's history).
- **`CAP_DAC_OVERRIDE` — not inherent to any ioctl, just a file-permission
  workaround.** It bypasses discretionary file-permission checks
  system-wide (not scoped to device files at all). It shows up because the
  device node (`/dev/sdX`, `/dev/nvme0`, ...) isn't owned/grouped in a way
  the running user already has access to, so plain `open()` would `EACCES`
  without it.

What's replaceable vs. inherent:

- **`CAP_DAC_OVERRIDE` should be eliminated, not just contained.** It's not
  needed for the ioctls themselves, only for `open()` on the device node. A
  udev rule setting the device node's group to one the daemon's user
  belongs to (or `TAG+="uaccess"`, which integrates with `systemd-logind`
  to grant a dynamic ACL) gives narrow, standard DAC permission to exactly
  the device files this app touches — instead of a capability that can
  bypass permission checks on every file on the system.
- **`CAP_SYS_RAWIO`/`CAP_SYS_ADMIN` are essentially inherent** to doing raw
  ATA/SCSI/NVMe passthrough on Linux, which is fundamental to this
  project's design (it implements the TCG protocol stack itself rather than
  delegating to a higher-level driver — Linux's in-tree Opal support
  (`IOC_OPAL_*` ioctls) has its own fixed session/protocol handling and a
  narrower feature set, doesn't help on Windows, and likely still requires
  `CAP_SYS_ADMIN` anyway, so it isn't a realistic substitute here). The goal
  for these two is containment, not elimination, once they're confined to
  the small daemon binary (already the plan via per-executable `setcap`
  rather than the whole GUI):
  - `CapabilityBoundingSet=CAP_SYS_RAWIO CAP_SYS_ADMIN` in the systemd unit
    — the process can never hold more than this, even via a later `exec`.
  - `NoNewPrivileges=yes` — blocks privilege escalation via `exec`.
  - `DeviceAllow=` (cgroup device controller) — allow-list only the
    block/NVMe device major/minor classes actually needed. Real
    kernel-enforced containment independent of the capability model: even
    holding these capabilities, a compromised process can't touch a device
    outside the allow-list.
  - `SystemCallFilter=` (seccomp) — allow-list `ioctl`/`open`/`read`/
    `write`/`close` plus whatever the RPC transport needs; deny `@mount`,
    `@module`, `@reboot`, `@raw-io` (blocks `iopl`/`ioperm`/`pciconfig_*`,
    which `CAP_SYS_RAWIO` would otherwise permit but this app never uses).
  - `systemd-analyze security <unit>` once the unit exists, for a concrete
    checklist instead of guesswork.
- Confirm empirically which capability ties to which drive backend
  (`auditd`/`journalctl -k` show `capable: ... CAP_SYS_ADMIN`-style denials
  when a capability is missing) before finalizing the file-capability set —
  the ATA-path-vs-NVMe-path split above is inferred from known kernel
  behavior, not traced against this exact codebase.

#### Threat model: what a compromised daemon can and can't do

Even with `CAP_SYS_RAWIO`/`CAP_SYS_ADMIN` plus the containment above, split
into what's actually blocked vs. not:

**Contained — lateral movement off the managed drive(s):**

- Mount/namespace manipulation, kernel module loading, `ptrace`-based
  injection into other processes, `bpf`/`perf_event_open`-based kernel
  attacks — blocked by `SystemCallFilter=` denying the relevant syscalls
  (`@mount`, `@module`, `ptrace`, `bpf`, ...) and by
  `CapabilityBoundingSet=` preventing the process from ever holding
  capabilities beyond these two.
- `/dev/mem`/`/dev/kmem` reads, raw I/O port access (`iopl`/`ioperm`) —
  `CAP_SYS_RAWIO` nominally allows this, but `DeviceAllow=` excludes those
  nodes and `SystemCallFilter=` can deny the `@raw-io` group directly.
- Any disk *not* in `DeviceAllow=` — enforced by the cgroup device
  controller independent of capabilities held.
- **Network exfiltration.** The daemon has no legitimate network need at
  all by design (telemetry is deliberately kept out of it; IPC is a local
  Unix socket). `RestrictAddressFamilies=AF_UNIX` (or `PrivateNetwork=yes`)
  blocks this with high confidence at zero functional cost — add
  unconditionally.

**Not contained — raw block I/O doesn't distinguish TCG commands from
anything else.** `CAP_SYS_RAWIO`/`CAP_SYS_ADMIN` plus an open fd on a block
device allow *any* command the passthrough interface supports, not just
`SECURITY PROTOCOL IN/OUT` or `NVME_IOCTL_ADMIN_CMD`'s Security Send/Receive
— including plain `READ`/`WRITE` at arbitrary LBAs, bypassing the
filesystem layer (and every file permission on that disk) entirely. No OS
mechanism scopes this down to "TCG opcodes only": not capabilities, not
cgroups, and not SELinux/AppArmor `ioctl` allow-listing either — LSM
`ioctl` extended permissions filter on the `ioctl(2)` request number (e.g.
"may call `SG_IO`"), not on the SCSI opcode byte inside that ioctl's
argument struct, so the distinction is invisible below the application
layer. (The only way to actually enforce it would be a custom eBPF-LSM hook
parsing the command buffer per call — a bespoke, kernel-version-fragile
undertaking, not standard hardening; not planned.)

Consequence: if `DeviceAllow=` includes the disk the user's live filesystem
is on (the common case — people mainly run this on their working/boot
drive), a fully compromised daemon can read every unlocked byte on that
disk and overwrite it in place. This isn't a gap the systemd directives
close; it's inherent to granting raw block I/O to a disk that also holds
user data.

**Reframing the acceptable/unacceptable line:** "the drive gets wiped" and
"the drive's live contents get read or corrupted" are the *same*
underlying primitive (raw write and raw read through the identical
passthrough interface), not different points on a severity scale — accepting
one as in-scope (given a bug or user mistake could cause it anyway) means
accepting the other too, since there's no design that keeps revert/erase
working while preventing it. The line that *is* achievable and worth
holding firmly: network-mediated exfiltration and remote-coordinated
ransom, both blocked by denying network access entirely. A ransomware
payload that never touches the network (encrypts in place, demands payment
out-of-band) would still be technically possible, but that's a narrower,
less realistic threat than the network-dependent version most real attacks
rely on.

**Bottom line for what this project actually buys:** confines a compromised
dependency to raw access of the specific drive(s) being managed, and
removes network egress and lateral system access. It does **not** protect
the contents of the managed drive itself — that's inherent to the feature
set (revert/erase needs raw write access), not a containment gap to be
closed later.

#### Windows: privilege is much harder to narrow than on Linux

"Run as Administrator" is currently required because the relevant IOCTLs —
`IOCTL_ATA_PASS_THROUGH`(`_DIRECT`), `IOCTL_SCSI_PASS_THROUGH`(`_DIRECT`),
and `IOCTL_STORAGE_PROTOCOL_COMMAND` (used for NVMe Security Send/Receive)
— are documented by Microsoft as requiring administrative privileges, and
this is enforced in the storage driver stack (`partmgr`/`classpnp`/
`storport`) as a check for the Administrators group SID being enabled on
the token — not a separately named, individually-grantable privilege the
way `CAP_SYS_RAWIO` is on Linux. Windows does have a fixed list of ~35
named privileges (`SeBackupPrivilege`, `SeLoadDriverPrivilege`,
`SeDebugPrivilege`, ...) that can be granted independently of full
Administrators membership, but none of them are documented as sufficient
for these specific IOCTLs. This is a deliberate anti-bypass measure (raw
sector writes to a mounted volume could otherwise circumvent NTFS ACLs),
the same underlying concern as the Linux `CAP_SYS_RAWIO` finding above —
just closed off by requiring full admin instead of a scoped capability.

**This is structurally worse than Linux, not just equally coarse.**
Removing `CAP_DAC_OVERRIDE` on Linux genuinely narrowed the daemon: raw
device I/O stayed, but the ability to bypass arbitrary file permissions
system-wide went away. There's no equivalent split on Windows —
"Administrators" is a single bundled grant, and it's the same SID that
(via default ACLs on almost every file and registry key) already gives
ambient read/write access to most of the filesystem and registry,
independent of raw disk access entirely. A daemon that satisfies the
storage-IOCTL gate is, by the same token, already able to read/write most
user files directly through normal file APIs — there's no way to keep
"may call the passthrough IOCTL" while shedding "is a general
administrator of this machine," because the OS checks the same underlying
fact for both.

What's still worth doing, in descending order of confidence:

1. **Still split the daemon from the GUI.** Keeps Slint + the OTLP/`tonic`
   dependency tree out of the admin-token process, even though the admin
   token itself can't be narrowed the way Linux's capability set can. This
   is still the main win and transfers directly.
2. **Strip every other privilege the token doesn't need**, via
   `CreateRestrictedToken`/`AdjustTokenPrivileges` at daemon startup. A
   default admin token typically has `SeDebugPrivilege`,
   `SeLoadDriverPrivilege`, `SeTakeOwnershipPrivilege`,
   `SeBackupPrivilege`/`SeRestorePrivilege`, `SeSecurityPrivilege`,
   `SeShutdownPrivilege` available — none needed for storage I/O. Disabling
   these doesn't touch the Administrators SID (so the storage IOCTL gate
   still passes), but it closes off unrelated admin powers: no loading a
   malicious driver, no debug-privilege injection into other processes, no
   taking ownership of arbitrary objects, no tampering with audit policy.
   Closest Windows analogue to `CapabilityBoundingSet=`; cheap to
   implement.
3. **Block network egress via a Windows Firewall rule scoped to the daemon
   executable** — best-effort, not a hard guarantee like Linux's
   `PrivateNetwork=`/seccomp: a fully compromised admin-token process can
   reconfigure Windows Firewall rules for itself, since firewall policy
   changes are gated by the same "is Administrator" check. Still raises the
   bar against unsophisticated payloads; real defense here would need
   something enforced from outside the daemon's own reach (a separate
   filtering driver, or perimeter/router-level blocking) — a known gap, not
   a solved problem.
4. **Put the GUI+telemetry process in an AppContainer / Low-Privilege
   AppContainer** (Windows' native sandbox, used by UWP apps and Chromium's
   sandbox). Not applicable to the daemon (AppContainers can't hold admin
   rights at all), but it's the right tool for the *unprivileged* side —
   shrinks the GUI/OTLP process's blast radius below a normal user process,
   which Linux doesn't get for free either. Complementary, independent of
   the elevation question.
5. **Verify empirically before accepting the above as final.** This is
   documentation-level knowledge and general driver-stack behavior, not a
   traced test against this exact IOCTL. It's possible (though unlikely,
   given how consistently the docs phrase it) that the check is actually
   DACL-based rather than a hard group-membership check, in which case
   granting a custom ACE on the specific device object to a non-admin
   service account might work without needing Administrators at all —
   worth a quick test (a restricted token with a custom-granted ACE on
   `\\.\PhysicalDriveN`, attempting the passthrough IOCTL) before
   concluding the Linux-style narrowing is impossible here.

Supply-chain hardening (`cargo audit`/`cargo deny` in CI, pinned and
reviewed dependency bumps) is an independent track — start it any time,
regardless of which stage this plan is at.
