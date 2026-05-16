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
    RevertSpResult, SessionMethodCall, SessionMethodCallParams, SessionMethodParam as _, SetBytes, SetObject,
    SetResult, extract_method,
};
use sed_spec::objects::{
    AccessControlRef, Ace, AceExpr, Authority, AuthorityRef, CPin, KAes256, KAes256Ref, LockingRange, MbrControl,
    MethodRef, SecurityProvider as SecurityProviderObj, SecurityProviderRef, TableDesc,
};
use sed_spec::preconfig::core::shared::invoking_id::THIS_SP;
use sed_spec::preconfig::core::shared::table_id;
use sed_spec::types::LifeCycleState;

use crate::tper::{Locking, SecurityProvider, Table, Tper};

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
        tper: &Tper,
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
    pub fn dispatch(&mut self, tper: &mut Tper, packet: Packet) -> Vec<(Packet, Vec<SecurityProviderRef>)> {
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
            .into_iter()
            .filter_map(|extract_result| match extract_result {
                ExtractResult::Ok { value, .. } => self.call(tper, value),
                ExtractResult::EndOfStream => self.close().map(|packet| (packet, vec![])),
                ExtractResult::NeedMoreTokens => None,
                ExtractResult::InvalidTokens(_) => self.abort().map(|packet| (packet, vec![])),
            })
            .collect()
    }

    fn call(&mut self, tper: &mut Tper, call: SessionMethodCall) -> Option<(Packet, Vec<SecurityProviderRef>)> {
        use SessionMethodCallParams::*;

        if let Self::Open { session_id, .. } = self {
            use SecurityProvider as Sp;

            let session_id = *session_id;

            let invoking_id = call.invoking_id;
            let (result_tokens, reverted_sps) = match call.params {
                Activate(params) => make_result(self.activate(tper, invoking_id, &params)),
                Authenticate(params) => make_result(self.authenticate(tper, invoking_id, &params)),
                GenKey(params) => make_result(self.gen_key(tper, invoking_id, &params)),
                Get(params) => make_result(self.get(tper, invoking_id, &params)),
                GetAcl(params) => make_result(self.get_acl(tper, invoking_id, &params)),
                Next(params) => make_result(self.next(tper, invoking_id, &params)),
                Random(params) => make_result(self.random(tper, invoking_id, &params)),
                Revert(params) => make_result_revert(self.revert(tper, invoking_id, &params)),
                RevertSp(params) => make_result_revert(self.revert_sp(tper, invoking_id, &params)),
                SetAce(p) => make_result(self.set_obj(tper, invoking_id, p, Sp::ace_mut)),
                SetAuthority(p) => make_result(self.set_obj(tper, invoking_id, p, Sp::authority_mut)),
                SetBytes(params) => make_result(self.set_bytes(tper, invoking_id, params)),
                SetCPin(p) => make_result(self.set_obj(tper, invoking_id, p, Sp::c_pin_mut)),
                SetKAes256(p) => make_result(self.set_obj_opt(tper, invoking_id, p, Sp::k_aes_256_mut)),
                SetLockingRange(p) => make_result(self.set_obj_opt(tper, invoking_id, p, Sp::locking_mut)),
                SetMbrControl(p) => make_result(self.set_obj_opt(tper, invoking_id, p, Sp::mbr_control_mut)),
                SetSecurityProvider(p) => make_result(self.set_obj_opt(tper, invoking_id, p, Sp::sp_mut)),
                SetTableDesc(p) => make_result(self.set_obj(tper, invoking_id, p, Sp::table_mut)),
            };
            tracing::debug!(method_result = tracing::field::debug(&result_tokens), "response");
            Some((
                session_id.assign(Packet {
                    payload: vec![SubPacket {
                        kind: SubPacketKind::Data,
                        length: std::marker::PhantomData,
                        payload: result_tokens,
                    }],
                    ..Default::default()
                }),
                reverted_sps,
            ))
        } else {
            None
        }
    }

    fn activate(&self, tper: &mut Tper, invoking_id: Uid, params: &Activate) -> Result<ActivateResult, MethodStatus> {
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
        tper: &mut Tper,
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

    fn gen_key(&self, tper: &mut Tper, invoking_id: Uid, params: &GenKey) -> Result<GenKeyResult, MethodStatus> {
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
        tper: &'tper mut Tper,
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

    fn get_acl(&self, tper: &mut Tper, invoking_id: Uid, params: &GetAcl) -> Result<GetAclResult, MethodStatus> {
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

    fn next(&self, tper: &mut Tper, invoking_id: Uid, params: &NextUntyped) -> Result<NextResultUntyped, MethodStatus> {
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

    fn random(&self, tper: &Tper, invoking_id: Uid, params: &Random) -> Result<RandomResult, MethodStatus> {
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

    fn revert(
        &mut self,
        tper: &mut Tper,
        invoking_id: Uid,
        params: &Revert,
    ) -> Result<(RevertResult, Vec<SecurityProviderRef>), MethodStatus> {
        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        let this_sp_uid = self.this_sp_uid().ok_or(MethodStatus::Fail)?;

        let sp_uid = SecurityProviderRef::try_from(invoking_id).map_err(|_| MethodStatus::InvalidParameter)?;
        let reverted_sps: Vec<_> = tper.restore_preconfig(sp_uid)?;
        if reverted_sps.contains(&this_sp_uid) {
            *self = Self::Closed;
        }
        Ok((RevertResult, reverted_sps))
    }

    fn revert_sp(
        &mut self,
        tper: &mut Tper,
        invoking_id: Uid,
        params: &RevertSp,
    ) -> Result<(RevertSpResult, Vec<SecurityProviderRef>), MethodStatus> {
        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        if invoking_id != THIS_SP {
            return Err(MethodStatus::InvalidParameter);
        }
        let this_sp_uid = self.this_sp_uid().ok_or(MethodStatus::Fail)?;

        // TODO: this parameters is currently ignored. It doesn't matter much
        // for the virtual device anyways.
        let _ = params.keep_global_range_key;
        let reverted_sps = tper.restore_preconfig(this_sp_uid)?;
        *self = Self::Closed;

        Ok((RevertSpResult, reverted_sps))
    }

    fn set_obj<'tper, O, G>(
        &self,
        tper: &'tper mut Tper,
        invoking_id: Uid,
        params: SetObject<O>,
        get_table_mut: G,
    ) -> Result<SetResult, MethodStatus>
    where
        O: Object,
        <O as Object>::Ref: TryFrom<Uid> + Ord + Copy,
        Uid: From<<O as Object>::Ref>,
        G: for<'a> FnOnce(&'a mut (dyn SecurityProvider + 'tper)) -> &'a mut Table<O>,
    {
        self.set_obj_opt(tper, invoking_id, params, |sp| Some(get_table_mut(sp)))
    }

    fn set_obj_opt<'tper, O, G>(
        &self,
        tper: &'tper mut Tper,
        invoking_id: Uid,
        params: SetObject<O>,
        get_table_mut: G,
    ) -> Result<SetResult, MethodStatus>
    where
        O: Object,
        <O as Object>::Ref: TryFrom<Uid> + Ord + Copy,
        Uid: From<<O as Object>::Ref>,
        G: for<'a> FnOnce(&'a mut (dyn SecurityProvider + 'tper)) -> Option<&'a mut Table<O>>,
    {
        if let Some(values) = &params.values {
            let columns = values.active_fields();
            self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), columns.into_iter())?;
        }

        let table_uid = invoking_id
            .is_table()
            .then_some(invoking_id)
            .or(invoking_id.containing_table())
            .or(params.where_.map(|where_| Uid::from(where_).containing_table()).flatten());
        let object_uid = <O as Object>::Ref::try_from(invoking_id).ok().or(params.where_);
        let (Some(table_uid), Some(object_uid)) = (table_uid, object_uid) else {
            return Err(MethodStatus::InvalidParameter);
        };
        if table_uid != Uid::from(object_uid).containing_table().expect("ObjectRefs always have a containing table") {
            return Err(MethodStatus::InvalidParameter);
        }

        let sp = self.this_sp_mut(tper)?;
        let table = get_table_mut(sp).ok_or(MethodStatus::InvalidParameter)?;
        let object = table.get_mut(&object_uid).ok_or(MethodStatus::InvalidParameter)?;

        if let Some(values) = params.values {
            object.update(values);
        }

        Ok(SetResult)
    }

    fn set_bytes(&self, tper: &mut Tper, invoking_id: Uid, params: SetBytes) -> Result<SetResult, MethodStatus> {
        self.check_permission(tper, invoking_id, params.method_id().try_into().unwrap(), [0].into_iter())?;

        let sp = self.this_sp_mut(tper)?;

        let table_uid = TableRef::try_from(invoking_id).map_err(|_| MethodStatus::InvalidParameter)?;
        let table = if table_uid == table_id::MBR {
            sp.mbr_mut().ok_or(MethodStatus::InvalidParameter)
        } else if table_id::DATA_STORE.contains(&table_uid) {
            let index = table_uid - table_id::DATA_STORE.start;
            sp.data_store_mut(index as usize).ok_or(MethodStatus::InvalidParameter)
        } else {
            Err(MethodStatus::InvalidParameter)
        }?;

        if let Some(values) = params.values {
            let range = params.where_.map(|x| x as usize).unwrap_or(0)..values.0.len();
            let slice = table.get_mut(range).ok_or(MethodStatus::InvalidParameter)?;
            slice.copy_from_slice(&values.0);
            Ok(SetResult)
        } else {
            Ok(SetResult)
        }
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

    pub fn this_sp_uid<'tper>(&self) -> Option<SecurityProviderRef> {
        match self {
            Session::Open { sp, .. } => Some(*sp),
            Session::Closed => None,
        }
    }

    fn this_sp<'tper>(&self, tper: &'tper Tper) -> Result<&'tper dyn SecurityProvider, MethodStatus> {
        let this_sp_uid = self.this_sp_uid().ok_or(MethodStatus::Fail)?;
        Ok(tper.sp(this_sp_uid).expect_sp(this_sp_uid))
    }

    fn this_sp_mut<'tper>(&self, tper: &'tper mut Tper) -> Result<&'tper mut dyn SecurityProvider, MethodStatus> {
        let this_sp_uid = self.this_sp_uid().ok_or(MethodStatus::Fail)?;
        Ok(tper.sp_mut(this_sp_uid).expect_sp(this_sp_uid))
    }

    fn check_permission(
        &self,
        tper: &Tper,
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

fn make_result<ResultList: Tokenize>(
    result_list: Result<ResultList, MethodStatus>,
) -> (Vec<u8>, Vec<SecurityProviderRef>) {
    (MethodResult(result_list).to_tokens().expect_serialize(), vec![])
}

fn make_result_revert<ResultList: Tokenize>(
    result_list: Result<(ResultList, Vec<SecurityProviderRef>), MethodStatus>,
) -> (Vec<u8>, Vec<SecurityProviderRef>) {
    match result_list {
        Ok((result_list, revert_list)) => (MethodResult(Ok(result_list)).to_tokens().expect_serialize(), revert_list),
        Err(err) => (MethodResult::<ResultList>(Err(err)).to_tokens().expect_serialize(), vec![]),
    }
}
