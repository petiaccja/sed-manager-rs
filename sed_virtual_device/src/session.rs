use std::collections::{BTreeMap, HashSet, VecDeque};
use std::ops::{Add, Bound, Range, RangeBounds};

use crate::internal_error::Expect;
use sed_packet::packet::{Packet, SubPacket, SubPacketKind};
use sed_packet::session_id::SessionId;
use sed_packet::token::{Command, ToTokens, Tokenize, Tokenizer};
use sed_packet::{Bytes, Object, ObjectRef, TableRef, TokenizeField, Uid};
use sed_spec::methods::{
    Activate, ActivateResult, Authenticate, AuthenticateResult, ByteCellBlock, CloseSession, ExtractResult, GenKey,
    GenKeyResult, Get, GetAcl, GetAclResult, MethodResult, MethodStatus, MgmtMethodCall, MgmtMethodCallParams,
    NextResultUntyped, NextUntyped, ObjectCellBlock, Random, RandomResult, Revert, RevertResult, RevertSp,
    RevertSpResult, SessionMethodCall, SessionMethodCallParams, SessionMethodParam as _, extract_method,
};
use sed_spec::objects::{
    AccessControlRef, Ace, AceExpr, Authority, AuthorityRef, CPin, KAes256, KAes256Ref, LockingRange, MbrControl,
    MethodRef, SecurityProvider as SecurityProviderObj, SecurityProviderRef, TableDesc,
};
use sed_spec::preconfig::core::shared::invoking_id::THIS_SP;
use sed_spec::preconfig::core::shared::table_id;
use sed_spec::types::LifeCycleState;

use crate::tper::{Locking, SecurityProvider, TPer, Table};

#[derive(Debug)]
pub enum Session {
    Open {
        session_id: SessionId,
        sp: SecurityProviderRef,
        authenticated: HashSet<AuthorityRef>,
        recv_buffer: VecDeque<u8>,
    },
    Closed,
}

impl Session {
    pub fn new(
        tper: &TPer,
        session_id: SessionId,
        sp_uid: SecurityProviderRef,
        authority_uid: AuthorityRef,
    ) -> Result<Self, MethodStatus> {
        let sp = tper.sp(sp_uid).ok_or(MethodStatus::InvalidParameter)?;
        let authority = sp.authority().get(&authority_uid).ok_or(MethodStatus::InvalidParameter)?;
        let authenticated = std::iter::once(authority_uid).chain(authority.class).collect();
        Ok(Self::Open { session_id, sp: sp_uid, authenticated, recv_buffer: VecDeque::new() })
    }

    #[must_use]
    pub fn dispatch(&mut self, tper: &mut TPer, packet: Packet) -> Vec<Packet> {
        if let Session::Open { recv_buffer, .. } = self {
            let data_sub_packets = packet.payload.iter().filter(|sub_packet| sub_packet.kind == SubPacketKind::Data);
            for sub_packet in data_sub_packets {
                recv_buffer.extend(sub_packet.payload.iter());
            }
        };

        let mut extracted_methods = Vec::new();
        if let Self::Open { recv_buffer, .. } = self {
            loop {
                match extract_method::<SessionMethodCall>(recv_buffer) {
                    value @ ExtractResult::Ok { .. } => extracted_methods.push(value),
                    ExtractResult::NeedMoreTokens => break,
                    value => {
                        extracted_methods.push(value);
                        break;
                    }
                }
            }
        }

        extracted_methods
            .iter()
            .filter_map(|extract_result| match &extract_result {
                ExtractResult::Ok { value, .. } => self.call(tper, value),
                ExtractResult::EndOfStream => self.close(),
                ExtractResult::NeedMoreTokens => None,
                ExtractResult::InvalidTokens(_) => self.abort(),
            })
            .collect()
    }

    fn call(&mut self, tper: &mut TPer, call: &SessionMethodCall) -> Option<Packet> {
        use SessionMethodCallParams::*;

        if let Self::Open { session_id, .. } = self {
            let session_id = *session_id;

            let invoking_id = call.invoking_id;
            let result_tokens = match &call.params {
                Activate(params) => MethodResult(self.activate(tper, invoking_id, params)).to_tokens(),
                Authenticate(params) => MethodResult(self.authenticate(tper, invoking_id, params)).to_tokens(),
                GenKey(params) => MethodResult(self.gen_key(tper, invoking_id, params)).to_tokens(),
                Get(params) => MethodResult(self.get(tper, invoking_id, params)).to_tokens(),
                GetAcl(params) => MethodResult(self.get_acl(tper, invoking_id, params)).to_tokens(),
                Next(params) => MethodResult(self.next(tper, invoking_id, params)).to_tokens(),
                Random(params) => MethodResult(self.random(tper, invoking_id, params)).to_tokens(),
                Revert(params) => MethodResult(self.revert(tper, invoking_id, params)).to_tokens(),
                RevertSp(params) => MethodResult(self.revert_sp(tper, invoking_id, params)).to_tokens(),
                SetAce(_params) => todo!(),
                SetAuthority(_params) => todo!(),
                SetBytes(_params) => todo!(),
                SetCPin(_params) => todo!(),
                SetKAes256(_params) => todo!(),
                SetLockingRange(_params) => todo!(),
                SetMbrControl(_params) => todo!(),
                SetSecurityProvider(_params) => todo!(),
                SetTableDesc(_params) => todo!(),
            };
            tracing::debug!(method_result = tracing::field::debug(&result_tokens), "response");
            Some(session_id.assign(Packet {
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: std::marker::PhantomData,
                    payload: result_tokens.expect_serialize(),
                }],
                ..Default::default()
            }))
        } else {
            None
        }
    }

    fn activate(&self, tper: &mut TPer, invoking_id: Uid, params: &Activate) -> Result<ActivateResult, MethodStatus> {
        use sed_spec::preconfig::opal_2::admin;
        use sed_spec::preconfig::opal_2::locking;

        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        let sp_uid = SecurityProviderRef::try_from(invoking_id).map_err(|_| MethodStatus::InvalidParameter)?;
        let admin_sp = tper.admin_sp_mut();
        let sid_pin = admin_sp.c_pin.get(&admin::c_pin::SID).expect_object("C_PIN", "SID").pin.clone();
        let sp_info = admin_sp.sp.get_mut(&sp_uid).ok_or(MethodStatus::InvalidParameter)?;
        if sp_info.life_cycle_state == Some(LifeCycleState::ManufacturedInactive) {
            sp_info.life_cycle_state = Some(LifeCycleState::Manufactured);
            let sp = tper.sp_mut(sp_uid).expect_sp(sp_uid);
            let admin1_pin =
                sp.c_pin_mut().get_mut(&locking::c_pin::ADMIN.get(1).unwrap()).expect_object("C_PIN", "ADMIN1");
            admin1_pin.pin = sid_pin;
            Ok(ActivateResult)
        } else {
            Err(MethodStatus::SPDisabled)
        }
    }

    fn authenticate(
        &mut self,
        tper: &mut TPer,
        invoking_id: Uid,
        params: &Authenticate,
    ) -> Result<AuthenticateResult, MethodStatus> {
        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        let this_sp = self.this_sp(tper)?;

        let Self::Open { authenticated, .. } = self else {
            return Err(MethodStatus::Fail);
        };

        let authority = this_sp.authority().get(&params.authority).ok_or(MethodStatus::InvalidParameter)?;
        if authority.is_class == Some(true) {
            return Err(MethodStatus::InvalidParameter);
        }
        if let Some(c_pin_uid) = authority.credential {
            let c_pin_uid = c_pin_uid.try_into().expect("internal error: non-PIN authentication");
            let c_pin = this_sp.c_pin().get(&c_pin_uid).expect_object("C_PIN", c_pin_uid);
            if c_pin.pin == params.proof {
                authenticated.insert(params.authority);
                if let Some(class) = authority.class {
                    authenticated.insert(class);
                }
                Ok(AuthenticateResult::Success(true))
            } else {
                Ok(AuthenticateResult::Success(false))
            }
        } else {
            authenticated.insert(params.authority);
            Ok(AuthenticateResult::Success(true))
        }
    }

    fn gen_key(&self, tper: &mut TPer, invoking_id: Uid, params: &GenKey) -> Result<GenKeyResult, MethodStatus> {
        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        let this_sp = self.this_sp_mut(tper)?;

        if let Ok(k_aes_256_uid) = KAes256Ref::try_from(invoking_id)
            && let Some(locking_sp) = this_sp.as_any_mut().downcast_mut::<Locking>()
            && params.pin_length.is_none()
            && params.public_exponent.is_none()
        {
            let k_aes_256 = locking_sp.k_aes_256.get_mut(&k_aes_256_uid).ok_or(MethodStatus::InvalidParameter)?;
            let mut new_key = [0u8; 64];
            rand::Fill::fill_slice(&mut new_key, &mut rand::rng());
            k_aes_256.key = Some(new_key);
            Ok(GenKeyResult)
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn get<'tper>(
        &self,
        tper: &'tper mut TPer,
        invoking_id: Uid,
        params: &Get,
    ) -> Result<GetResult<'tper>, MethodStatus> {
        fn get_slice<O: Object>(
            table: &Table<O>,
            object: Uid,
            start_column: Option<u16>,
            end_column: Option<u16>,
        ) -> Result<ObjectSlice<'_, O>, MethodStatus>
        where
            O::Ref: TryFrom<Uid> + Ord,
        {
            let object = table
                .get(&object.try_into().map_err(|_| MethodStatus::InvalidParameter)?)
                .ok_or(MethodStatus::InvalidParameter)?;
            let (start_field, end_field) = unmap_bounds(start_column, end_column);
            Ok(ObjectSlice { object, start_field, end_field })
        }

        let sp = self.this_sp(tper)?;
        if let Ok(ObjectCellBlock { table, object, start_column, end_column }) =
            params.cell_block.clone().try_into_object(invoking_id)
        {
            match table {
                table_id::ACE => get_slice(sp.ace(), object, start_column, end_column).map(|s| GetResult::Ace(s)),
                table_id::AUTHORITY => {
                    get_slice(sp.authority(), object, start_column, end_column).map(|s| GetResult::Authority(s))
                }
                table_id::C_PIN => get_slice(sp.c_pin(), object, start_column, end_column).map(|s| GetResult::CPin(s)),
                table_id::K_AES_256 => {
                    get_slice(sp.k_aes_256().ok_or(MethodStatus::InvalidParameter)?, object, start_column, end_column)
                        .map(|s| GetResult::KAes256(s))
                }
                table_id::LOCKING => {
                    get_slice(sp.locking().ok_or(MethodStatus::InvalidParameter)?, object, start_column, end_column)
                        .map(|s| GetResult::LockingRange(s))
                }
                table_id::MBR_CONTROL => {
                    get_slice(sp.mbr_control().ok_or(MethodStatus::InvalidParameter)?, object, start_column, end_column)
                        .map(|s| GetResult::MbrControl(s))
                }
                table_id::SP => {
                    get_slice(sp.sp().ok_or(MethodStatus::InvalidParameter)?, object, start_column, end_column)
                        .map(|s| GetResult::SecurityProvider(s))
                }
                table_id::TABLE => {
                    get_slice(sp.table(), object, start_column, end_column).map(|s| GetResult::TableDesc(s))
                }
                _ => Err(MethodStatus::InvalidParameter),
            }
        } else if let Ok(ByteCellBlock { table, start_byte, end_byte }) =
            params.cell_block.clone().try_into_byte(invoking_id)
        {
            let range = unmap_bounds(start_byte.map(|x| x as usize), end_byte.map(|x| x as usize));

            let table = if table == table_id::MBR {
                sp.mbr().ok_or(MethodStatus::InvalidParameter)
            } else if table_id::DATA_STORE.contains(&table) {
                let index = table - table_id::DATA_STORE.start;
                sp.data_store(index as usize).ok_or(MethodStatus::InvalidParameter)
            } else {
                Err(MethodStatus::InvalidParameter)
            }?;

            let slice = table.get(range).ok_or(MethodStatus::InvalidParameter)?;
            Ok(GetResult::Bytes(Bytes(slice.into())))
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn get_acl(&self, tper: &mut TPer, invoking_id: Uid, params: &GetAcl) -> Result<GetAclResult, MethodStatus> {
        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        if invoking_id == table_id::ACCESS_CONTROL.to_uid() {
            let this_sp = self.this_sp(tper)?;
            let ac = this_sp
                .access_control()
                .get(&AccessControlRef { invoking_id: params.invoking_id, method_id: params.method_id })
                .ok_or(MethodStatus::InvalidParameter)?;

            Ok(GetAclResult { acl: ac.acl.clone() })
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn next(&self, tper: &mut TPer, invoking_id: Uid, params: &NextUntyped) -> Result<NextResultUntyped, MethodStatus> {
        fn list_objects<const TABLE: u64, O>(
            table: Option<&BTreeMap<ObjectRef<TABLE>, O>>,
            where_: Option<Uid>,
            count: Option<u64>,
        ) -> Result<Vec<Uid>, MethodStatus> {
            let table = table.ok_or(MethodStatus::InvalidParameter)?;

            let range = if let Some(where_) = where_ {
                let where_ = ObjectRef::<TABLE>::try_from(where_).map_err(|_| MethodStatus::InvalidParameter)?;
                if !table.contains_key(&where_) {
                    return Err(MethodStatus::InvalidParameter);
                }
                (Bound::Excluded(where_), Bound::Unbounded)
            } else {
                (Bound::Unbounded, Bound::Unbounded)
            };

            let count = count.map(|count| count as usize).unwrap_or(usize::MAX);

            Ok(table.range(range).take(count).map(|(ref_, _)| ref_.to_uid()).collect())
        }

        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        let sp = self.this_sp(tper)?;
        let table = TableRef::try_from(invoking_id).map_err(|_| MethodStatus::InvalidParameter)?;

        let objects = match table {
            table_id::ACE => list_objects(Some(sp.ace()), params.where_, params.count)?,
            table_id::AUTHORITY => list_objects(Some(sp.authority()), params.where_, params.count)?,
            table_id::C_PIN => list_objects(Some(sp.c_pin()), params.where_, params.count)?,
            table_id::K_AES_256 => list_objects(sp.k_aes_256(), params.where_, params.count)?,
            table_id::LOCKING => list_objects(sp.locking(), params.where_, params.count)?,
            table_id::MBR_CONTROL => list_objects(sp.mbr_control(), params.where_, params.count)?,
            table_id::SP => list_objects(sp.sp(), params.where_, params.count)?,
            table_id::TABLE => list_objects(Some(sp.table()), params.where_, params.count)?,
            _ => return Err(MethodStatus::InvalidParameter),
        };

        Ok(NextResultUntyped { result: objects })
    }

    fn random(&self, tper: &TPer, invoking_id: Uid, params: &Random) -> Result<RandomResult, MethodStatus> {
        use rand::prelude::*;

        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        if invoking_id == THIS_SP {
            let mut rng = rand::rng();
            let mut bytes = Vec::new();
            bytes.resize_with(params.count as usize, || rng.random());
            Ok(RandomResult { result: Bytes(bytes) })
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    fn revert(&self, tper: &mut TPer, invoking_id: Uid, params: &Revert) -> Result<RevertResult, MethodStatus> {
        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        let sp_uid = SecurityProviderRef::try_from(invoking_id).map_err(|_| MethodStatus::InvalidParameter)?;
        tper.restore_preconfig(sp_uid)?;
        Ok(RevertResult)
    }

    fn revert_sp(&self, tper: &mut TPer, invoking_id: Uid, params: &RevertSp) -> Result<RevertSpResult, MethodStatus> {
        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        if invoking_id != THIS_SP {
            return Err(MethodStatus::InvalidParameter);
        }
        let sp_uid = match self {
            Session::Open { sp, .. } => *sp,
            Session::Closed => return Err(MethodStatus::Fail),
        };

        // TODO: this parameters is currently ignored. It doesn't matter much
        // for the virtual device anyways.
        let _ = params.keep_global_range_key;
        tper.restore_preconfig(sp_uid)?;
        Ok(RevertSpResult)
    }

    fn close(&mut self) -> Option<Packet> {
        if let Self::Open { session_id, .. } = self {
            let session_id = *session_id;
            *self = Self::Closed;

            let call = Command::EndOfSession;
            Some(session_id.assign(Packet {
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: std::marker::PhantomData,
                    payload: call.to_tokens().expect_tokenize(),
                }],
                ..Default::default()
            }))
        } else {
            None
        }
    }

    fn abort(&mut self) -> Option<Packet> {
        if let Self::Open { session_id, .. } = self {
            let session_id = *session_id;
            *self = Self::Closed;

            let call = MgmtMethodCall {
                params: MgmtMethodCallParams::CloseSession(CloseSession {
                    remote_session_number: session_id.hsn,
                    local_session_number: session_id.tsn,
                }),
                status: MethodStatus::Success,
            };
            Some(SessionId::MANAGEMENT.assign(Packet {
                payload: vec![SubPacket {
                    kind: SubPacketKind::Data,
                    length: std::marker::PhantomData,
                    payload: call.to_tokens().expect_tokenize(),
                }],
                ..Default::default()
            }))
        } else {
            None
        }
    }

    fn this_sp<'tper>(&self, tper: &'tper TPer) -> Result<&'tper dyn SecurityProvider, MethodStatus> {
        match self {
            Session::Open { sp, .. } => Ok(tper.sp(*sp).expect_sp(*sp)),
            Session::Closed => Err(MethodStatus::Fail),
        }
    }

    fn this_sp_mut<'tper>(&self, tper: &'tper mut TPer) -> Result<&'tper mut dyn SecurityProvider, MethodStatus> {
        match self {
            Session::Open { sp, .. } => Ok(tper.sp_mut(*sp).expect_sp(*sp)),
            Session::Closed => Err(MethodStatus::Fail),
        }
    }

    fn check_permission(
        &self,
        tper: &TPer,
        invoking_id: Uid,
        method_id: MethodRef,
        mut columns: impl Iterator<Item = u16>,
    ) -> Result<(), MethodStatus> {
        let Self::Open { authenticated, .. } = self else {
            return Err(MethodStatus::Fail);
        };
        let this_sp = self.this_sp(tper)?;
        let ac_table = this_sp.access_control();

        let mut permitted_columns = HashSet::new();
        // Check both the invoking ID and its containing table. ACLs for the
        // containing table apply to any object in the table.
        for invoking_id in std::iter::once(invoking_id).chain(invoking_id.containing_table()) {
            if let Some(access_control) = ac_table.get(&AccessControlRef { invoking_id, method_id }) {
                let ace_table = this_sp.ace();
                for ace_ref in &access_control.acl {
                    let ace = ace_table.get(ace_ref).expect_object("ACE", ace_ref);
                    let has_permission = ace
                        .boolean_expr
                        .as_ref()
                        .map(|expr| expr.eval(authenticated.iter().cloned()))
                        .flatten()
                        .unwrap_or(false);
                    if has_permission {
                        permitted_columns.extend(ace.columns.as_ref().unwrap_or(&HashSet::new()).iter().cloned());
                    }
                }
            }
        }

        if columns.all(|column| permitted_columns.contains(&column)) {
            Ok(())
        } else {
            Err(MethodStatus::NotAuthorized)
        }
    }
}

struct ObjectSlice<'o, O> {
    object: &'o O,
    start_field: Bound<u16>,
    end_field: Bound<u16>,
}

impl<'o, O> Tokenize for ObjectSlice<'o, O>
where
    O: TokenizeField + Object,
{
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_list(|tokenizer| {
            let fields = normalize_bounds(self.start_field, self.end_field, 0, O::FIELD_COUNT);
            for field in fields {
                self.object.tokenize_field(field, tokenizer)?;
            }
            Ok(())
        })
    }
}

enum GetResult<'tper> {
    Ace(ObjectSlice<'tper, Ace>),
    Authority(ObjectSlice<'tper, Authority>),
    CPin(ObjectSlice<'tper, CPin>),
    KAes256(ObjectSlice<'tper, KAes256>),
    LockingRange(ObjectSlice<'tper, LockingRange>),
    MbrControl(ObjectSlice<'tper, MbrControl>),
    SecurityProvider(ObjectSlice<'tper, SecurityProviderObj>),
    TableDesc(ObjectSlice<'tper, TableDesc>),
    Bytes(Bytes),
}

impl Tokenize for GetResult<'_> {
    fn tokenize<T: Tokenizer>(&self, tokenizer: &mut T) -> Result<(), T::Error> {
        tokenizer.tokenize_list(|tokenizer| match self {
            GetResult::Ace(value) => value.tokenize(tokenizer),
            GetResult::Authority(value) => value.tokenize(tokenizer),
            GetResult::CPin(value) => value.tokenize(tokenizer),
            GetResult::KAes256(value) => value.tokenize(tokenizer),
            GetResult::LockingRange(value) => value.tokenize(tokenizer),
            GetResult::MbrControl(value) => value.tokenize(tokenizer),
            GetResult::SecurityProvider(value) => value.tokenize(tokenizer),
            GetResult::TableDesc(value) => value.tokenize(tokenizer),
            GetResult::Bytes(value) => value.tokenize(tokenizer),
        })
    }
}

fn unmap_bounds<T>(start: Option<T>, end: Option<T>) -> (Bound<T>, Bound<T>) {
    let start = match start {
        None => Bound::Unbounded,
        Some(x) => Bound::Included(x),
    };
    let end = match end {
        None => Bound::Unbounded,
        Some(x) => Bound::Included(x),
    };
    (start, end)
}

fn normalize_bounds<T: Add<T, Output = T> + From<u8>>(start: Bound<T>, end: Bound<T>, min: T, max: T) -> Range<T> {
    let start = match start {
        Bound::Included(x) => x,
        Bound::Excluded(x) => x + T::from(1u8),
        Bound::Unbounded => min,
    };

    let end = match end {
        Bound::Included(x) => x + T::from(1u8),
        Bound::Excluded(x) => x,
        Bound::Unbounded => max,
    };

    start..end
}
