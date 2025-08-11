//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::applications::{get_admin_sp, get_locking_sp};
use crate::messaging::discovery::FeatureCode;
use crate::spec::column_types::{AuthMethod, AuthorityRef, LifeCycleState, SPRef};
use crate::spec::objects::{Authority, SP};
use crate::spec::{self, table_id};
use crate::tper::{Session, TPer};

use super::Error;

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
    use crate::applications::test_fixtures::{setup_activated_tper, setup_factory_tper};
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
}
