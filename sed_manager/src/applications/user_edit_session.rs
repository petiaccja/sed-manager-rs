use crate::{
    spec::{
        column_types::{AuthMethod, AuthorityRef, CPINRef},
        objects::{Authority, CPIN},
        table_id,
    },
    tper::{Session, TPer},
};

use super::{utility::start_admin1_session, Error};

pub struct UserEditSession {
    session: Session,
}

impl UserEditSession {
    pub async fn start(tper: &TPer, admin1_password: &[u8]) -> Result<Self, Error> {
        Ok(Self { session: start_admin1_session(tper, admin1_password).await? })
    }

    pub async fn end(self) -> Result<(), Error> {
        Ok(self.session.end_session().await?)
    }

    pub async fn list_users(&self) -> Result<Vec<AuthorityRef>, Error> {
        let authorities = self.session.next(table_id::AUTHORITY, None, None).await?;
        let authorities = authorities.into_iter().filter_map(|uid| AuthorityRef::try_from(uid).ok());
        let mut users = Vec::new();
        for authority in authorities {
            if self.is_user(authority).await? {
                users.push(authority);
            }
        }
        Ok(users)
    }

    pub async fn get_user(&self, user: AuthorityRef) -> Result<Authority, Error> {
        let common_name = self.session.get(user.as_uid(), Authority::COMMON_NAME).await?;
        let enabled = self.session.get(user.as_uid(), Authority::ENABLED).await?;

        Ok(Authority { uid: user, common_name, enabled, ..Default::default() })
    }

    pub async fn set_enabled(&self, user: AuthorityRef, enabled: bool) -> Result<(), Error> {
        Ok(self.session.set(user.as_uid(), Authority::ENABLED, enabled).await?)
    }

    pub async fn set_name(&self, user: AuthorityRef, name: &str) -> Result<(), Error> {
        Ok(self.session.set(user.as_uid(), Authority::COMMON_NAME, name.as_bytes()).await?)
    }

    pub async fn set_password(&self, user: AuthorityRef, password: &[u8]) -> Result<(), Error> {
        let credential: CPINRef = self.session.get(user.as_uid(), Authority::CREDENTIAL).await?;
        Ok(self.session.set(credential.as_uid(), CPIN::PIN, password).await?)
    }

    async fn is_user(&self, authority: AuthorityRef) -> Result<bool, Error> {
        let operation: AuthMethod = self.session.get(authority.as_uid(), Authority::OPERATION).await?;
        Ok(operation == AuthMethod::Password)
    }
}

#[cfg(test)]
mod tests {
    use crate::{applications::utility::tests::setup_activated_tper, fake_device::MSID_PASSWORD, spec};

    use super::*;

    #[tokio::test]
    async fn list_users() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = UserEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let users = session.list_users().await?;
        assert_eq!(users.len(), 12);
        assert!(users.contains(&spec::opal::locking::authority::ADMIN.nth(1).unwrap()));
        assert!(users.contains(&spec::opal::locking::authority::ADMIN.nth(4).unwrap()));
        assert!(users.contains(&spec::opal::locking::authority::USER.nth(1).unwrap()));
        assert!(users.contains(&spec::opal::locking::authority::USER.nth(8).unwrap()));
        Ok(())
    }

    #[tokio::test]
    async fn set_enabled() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = UserEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let user = spec::opal::locking::authority::USER.nth(2).unwrap();
        session.set_enabled(user, true).await?;
        let user = session.get_user(user).await?;
        assert_eq!(user.enabled, true);
        Ok(())
    }

    #[tokio::test]
    async fn set_name() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = UserEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let user = spec::opal::locking::authority::USER.nth(2).unwrap();
        session.set_name(user, "Winnie the Pooh").await?;
        let user = session.get_user(user).await?;
        assert_eq!(user.common_name.as_slice(), "Winnie the Pooh".as_bytes());
        Ok(())
    }

    #[tokio::test]
    async fn set_password() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let user = spec::opal::locking::authority::USER.nth(2).unwrap();
        {
            let session = UserEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
            session.set_enabled(user, true).await?;
            session.set_password(user, "12345".as_bytes()).await?;
            session.end().await?;
        }
        let result = tper.start_session(spec::opal::admin::sp::LOCKING, Some(user), Some("12345".as_bytes())).await;
        assert!(result.map(|_| ()) == Ok(()));
        Ok(())
    }
}
