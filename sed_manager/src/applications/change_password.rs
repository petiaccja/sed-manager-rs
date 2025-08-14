//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::applications::{get_admin_sp, get_locking_sp};
use crate::messaging::discovery::FeatureCode;
use crate::spec::column_types::{AuthMethod, AuthorityRef, LifeCycleState, SPRef};
use crate::spec::objects::{Authority, CPIN, SP};
use crate::spec::{self, table_id};
use crate::tper::{Session, TPer};

use super::Error;

pub async fn change_password(
    tper: &TPer,
    sp: SPRef,
    authority: AuthorityRef,
    password: &[u8],
    new_password: &[u8],
) -> Result<(), Error> {
    let session = tper.start_session(sp, Some(authority), Some(password)).await?;
    session
        .with(async |session| {
            let credential = if let Some(idx) = spec::opal::locking::authority::USER.index_of(authority) {
                // Unfortunately, User#N authorities don't have an ACE to query their own C_PIN credential.
                spec::opal::locking::c_pin::USER.nth(idx).ok_or(Error::InternalError)?
            } else {
                session.get(authority.as_uid(), Authority::CREDENTIAL).await?
            };
            session.set(credential.as_uid(), CPIN::PIN, new_password).await.map_err(|err| err.into())
        })
        .await
}

pub fn is_change_password_supported() -> bool {
    true
}

async fn get_active_security_providers(tper: &TPer) -> Result<Vec<SPRef>, Error> {
    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::NoAvailableSSC)?;
    if ssc.feature_code() == FeatureCode::Enterprise {
        // Enterprise doesn't have an SP table, but the Locking SP is always issued.
        Ok(vec![
            get_admin_sp(ssc.feature_code())?,
            get_locking_sp(ssc.feature_code())?,
        ])
    } else {
        let admin_sp = get_admin_sp(ssc.feature_code())?;
        let session = tper.start_session(admin_sp, None, None).await?;
        session
            .with(async |session| {
                let sps = session.next(table_id::SP, None, None).await?;
                let mut active_sps = Vec::new();
                for sp in sps {
                    let life_cycle_state: LifeCycleState = session.get(sp, SP::LIFE_CYCLE_STATE).await?;
                    if [LifeCycleState::Issued, LifeCycleState::Manufactured].contains(&life_cycle_state) {
                        active_sps.push(SPRef::try_from(sp).unwrap());
                    }
                }
                Ok(active_sps)
            })
            .await
    }
}

async fn is_password_authority(
    session: &Session,
    auth: AuthorityRef,
    ssc: FeatureCode,
    sp: SPRef,
) -> Result<bool, Error> {
    use spec::opal::admin::sp::LOCKING;
    use FeatureCode::*;

    if [OpalV1, OpalV2, Opalite, PyriteV1, PyriteV2, Ruby, KeyPerIO].contains(&ssc) && sp == LOCKING {
        // On these SSCs, Anybody can enumerate the Authority table, but not Get its objects.
        let admins = &spec::opal::locking::authority::ADMIN;
        let users = &spec::opal::locking::authority::USER;
        let is_admin_or_user = admins.contains(auth) || users.contains(auth);
        let is_class = auth == spec::opal::locking::authority::ADMINS || auth == spec::opal::locking::authority::USERS;
        Ok(is_admin_or_user && !is_class)
    } else {
        let auth_method: AuthMethod = session.get(auth.as_uid(), Authority::OPERATION).await?;
        Ok(auth_method == AuthMethod::Password)
    }
}

pub async fn get_password_authorities(tper: &TPer, sp: SPRef) -> Result<Vec<AuthorityRef>, Error> {
    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::NoAvailableSSC)?;

    if ssc.feature_code() == FeatureCode::Enterprise && sp == spec::enterprise::admin::sp::ADMIN {
        // On Enterprise, the Admin::Authority table is not enumerable by Anybody,
        // so we must resort to hardcoding the relevant authorities.
        Ok(vec![spec::enterprise::admin::authority::SID])
    } else {
        let session = tper.start_session(sp, None, None).await?;
        session
            .with(async |session| {
                let auths = session.next(table_id::AUTHORITY, None, None).await?;
                let mut password_auths = Vec::new();
                for auth in auths {
                    if is_password_authority(&session, auth.try_into().unwrap(), ssc.feature_code(), sp).await? {
                        password_auths.push(auth.try_into().unwrap());
                    }
                }
                Ok(password_auths)
            })
            .await
    }
}

pub async fn list_password_authorities(tper: &TPer) -> Result<Vec<(SPRef, AuthorityRef)>, Error> {
    let mut collected = Vec::new();

    let security_providers = get_active_security_providers(tper).await?;
    for sp in security_providers {
        let auths = get_password_authorities(tper, sp).await?;
        for auth in auths {
            collected.push((sp, auth));
        }
    }

    Ok(collected)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::applications::test_fixtures::{make_activated_device, setup_activated_tper, setup_factory_tper};
    use crate::fake_device::data::object_table::{AuthorityTable, CPINTable};
    use crate::fake_device::FakeDevice;
    use crate::rpc::{MethodStatus, TokioRuntime};
    use crate::spec::column_types::CPINRef;
    use crate::spec::{opal, psid};

    use super::*;

    #[tokio::test]
    async fn get_active_sps_factory() -> Result<(), Error> {
        let tper = setup_factory_tper();
        let mut authorities = get_active_security_providers(&tper).await?;
        let mut expected = [opal::admin::sp::ADMIN];
        authorities.sort();
        expected.sort();
        assert_eq!(authorities, expected);
        Ok(())
    }

    #[tokio::test]
    async fn get_active_sps_activated() -> Result<(), Error> {
        let tper = setup_activated_tper();
        let mut authorities = get_active_security_providers(&tper).await?;
        let mut expected = [opal::admin::sp::ADMIN, opal::admin::sp::LOCKING];
        authorities.sort();
        expected.sort();
        assert_eq!(authorities, expected);
        Ok(())
    }

    #[tokio::test]
    async fn get_password_auths_admin() -> Result<(), Error> {
        let tper = setup_activated_tper();
        let mut authorities = get_password_authorities(&tper, opal::admin::sp::ADMIN).await?;
        let mut expected = [
            opal::admin::authority::SID,
            opal::admin::authority::ADMIN.nth(1).unwrap(),
            opal::admin::authority::ADMIN.nth(2).unwrap(),
            opal::admin::authority::ADMIN.nth(3).unwrap(),
            opal::admin::authority::ADMIN.nth(4).unwrap(),
            psid::admin::authority::PSID,
        ];
        authorities.sort();
        expected.sort();
        assert_eq!(authorities, expected);
        Ok(())
    }

    #[tokio::test]
    async fn list_authorities_factory() -> Result<(), Error> {
        let tper = setup_factory_tper();
        let mut authorities = list_password_authorities(&tper).await?;
        let mut expected = [
            (opal::admin::sp::ADMIN, opal::admin::authority::SID),
            (opal::admin::sp::ADMIN, opal::admin::authority::ADMIN.nth(1).unwrap()),
            (opal::admin::sp::ADMIN, opal::admin::authority::ADMIN.nth(2).unwrap()),
            (opal::admin::sp::ADMIN, opal::admin::authority::ADMIN.nth(3).unwrap()),
            (opal::admin::sp::ADMIN, opal::admin::authority::ADMIN.nth(4).unwrap()),
            (opal::admin::sp::ADMIN, psid::admin::authority::PSID),
        ];
        authorities.sort();
        expected.sort();
        assert_eq!(authorities, expected);
        Ok(())
    }

    #[tokio::test]
    async fn list_authorities_activated() -> Result<(), Error> {
        let tper = setup_activated_tper();
        let mut authorities = list_password_authorities(&tper).await?;
        let mut expected = [
            (opal::admin::sp::ADMIN, opal::admin::authority::SID),
            (opal::admin::sp::ADMIN, opal::admin::authority::ADMIN.nth(1).unwrap()),
            (opal::admin::sp::ADMIN, opal::admin::authority::ADMIN.nth(2).unwrap()),
            (opal::admin::sp::ADMIN, opal::admin::authority::ADMIN.nth(3).unwrap()),
            (opal::admin::sp::ADMIN, opal::admin::authority::ADMIN.nth(4).unwrap()),
            (opal::admin::sp::ADMIN, psid::admin::authority::PSID),
            (opal::admin::sp::LOCKING, opal::locking::authority::ADMIN.nth(1).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::ADMIN.nth(2).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::ADMIN.nth(3).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::ADMIN.nth(4).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::USER.nth(1).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::USER.nth(2).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::USER.nth(3).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::USER.nth(4).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::USER.nth(5).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::USER.nth(6).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::USER.nth(7).unwrap()),
            (opal::admin::sp::LOCKING, opal::locking::authority::USER.nth(8).unwrap()),
        ];
        authorities.sort();
        expected.sort();
        assert_eq!(authorities, expected);
        Ok(())
    }

    fn get_pin(device: &FakeDevice, sp_ref: SPRef, authority_ref: AuthorityRef) -> Vec<u8> {
        device.with_tper(|tper| {
            let sp = tper.ssc.get_sp(sp_ref).unwrap();
            let authorities: &AuthorityTable = sp.get_object_table_specific(table_id::AUTHORITY).unwrap();
            let authority = authorities.get(&authority_ref).unwrap();
            let credential_ref = authority.credential;
            let c_pins: &CPINTable = sp.get_object_table_specific(table_id::C_PIN).unwrap();
            let credential = c_pins.get(&CPINRef::try_from(credential_ref.as_uid()).unwrap()).unwrap();
            credential.pin.to_vec()
        })
    }

    #[tokio::test]
    async fn change_password_user1_success() -> Result<(), Error> {
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(make_activated_device());
        let tper = TPer::new_on_default_com_id(device.clone(), runtime)?;
        let authority = opal::locking::authority::USER.nth(1).unwrap();
        let sp = opal::admin::sp::LOCKING;
        let password = get_pin(&*device, sp, authority);
        let new_password = "kjgfjs".as_bytes();
        change_password(&tper, sp, authority, &password, new_password).await?;
        assert_eq!(get_pin(&*device, sp, authority), new_password);
        Ok(())
    }

    #[tokio::test]
    async fn change_password_admin1_success() -> Result<(), Error> {
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(make_activated_device());
        let tper = TPer::new_on_default_com_id(device.clone(), runtime)?;
        let authority = opal::locking::authority::ADMIN.nth(1).unwrap();
        let sp = opal::admin::sp::LOCKING;
        let password = get_pin(&*device, sp, authority);
        let new_password = "kjgfjs".as_bytes();
        change_password(&tper, sp, authority, &password, new_password).await?;
        assert_eq!(get_pin(&*device, sp, authority), new_password);
        Ok(())
    }

    #[tokio::test]
    async fn change_password_failed_authentication() -> Result<(), Error> {
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(make_activated_device());
        let tper = TPer::new_on_default_com_id(device.clone(), runtime)?;
        let authority = opal::locking::authority::USER.nth(1).unwrap();
        let sp = opal::admin::sp::LOCKING;
        let password = "luvcgw".as_bytes();
        let new_password = "kjgfjs".as_bytes();
        assert_eq!(
            change_password(&tper, sp, authority, &password, new_password).await,
            Err(Error::RPCError(MethodStatus::NotAuthorized.into()))
        );
        Ok(())
    }
}
