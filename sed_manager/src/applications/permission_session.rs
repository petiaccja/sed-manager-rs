use crate::messaging::discovery::LockingDescriptor;
use crate::rpc::Error as RPCError;
use crate::spec::basic_types::List;
use crate::spec::column_types::{ACEOperand, ACERef, AuthorityRef, BooleanOp, LockingRangeRef};
use crate::spec::objects::{Authority, LockingRange, ACE};
use crate::spec::{self, method_id, table_id};
use crate::tper::{Session, TPer};

use super::{utility::start_admin1_session, Error};

pub struct PermissionEditSession {
    session: Session,
    is_mbr_supported: bool,
}

impl PermissionEditSession {
    pub async fn start(tper: &TPer, admin1_password: &[u8]) -> Result<Self, Error> {
        let discovery = tper.discover().await?;
        let locking_desc = discovery.get::<LockingDescriptor>().ok_or(Error::IncompatibleSSC)?;
        let is_mbr_supported = !locking_desc.mbr_shadowing_not_supported;
        Ok(Self { session: start_admin1_session(tper, admin1_password).await?, is_mbr_supported })
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

    pub async fn list_ranges(&self) -> Result<Vec<LockingRangeRef>, Error> {
        let ranges = self.session.next(table_id::LOCKING, None, None).await?;
        Ok(ranges.into_iter().filter_map(|uid| LockingRangeRef::try_from(uid).ok()).collect())
    }

    pub async fn is_mbr_supported(&self) -> bool {
        self.is_mbr_supported
    }

    pub async fn get_mbr_permission(&self, user: AuthorityRef) -> Result<bool, Error> {
        if !self.is_mbr_supported().await {
            return Ok(false);
        } else {
            let ace = spec::opal::locking::ace::MBR_CONTROL_SET_DONE_TO_DOR; // The same for all relevant SSCs.
            let expr: List<ACEOperand> = self.session.get(ace.as_uid(), ACE::BOOLEAN_EXPR).await?;
            Ok(has_permission(&expr, user))
        }
    }

    pub async fn get_read_permission(&self, user: AuthorityRef, range: LockingRangeRef) -> Result<bool, Error> {
        let ace = self.get_ace(range, LockingRange::READ_LOCKED).await?;
        let expr: List<ACEOperand> = self.session.get(ace.as_uid(), ACE::BOOLEAN_EXPR).await?;
        Ok(has_permission(&expr, user))
    }

    pub async fn get_write_permission(&self, user: AuthorityRef, range: LockingRangeRef) -> Result<bool, Error> {
        let ace = self.get_ace(range, LockingRange::WRITE_LOCKED).await?;
        let expr: List<ACEOperand> = self.session.get(ace.as_uid(), ACE::BOOLEAN_EXPR).await?;
        Ok(has_permission(&expr, user))
    }

    pub async fn set_read_permission(
        &self,
        _user: AuthorityRef,
        _range: LockingRangeRef,
        _permitted: bool,
    ) -> Result<(), Error> {
        Err(RPCError::NotImplemented.into())
    }

    pub async fn set_write_permission(
        &self,
        _user: AuthorityRef,
        _range: LockingRangeRef,
        _permitted: bool,
    ) -> Result<(), Error> {
        Err(RPCError::NotImplemented.into())
    }

    async fn get_ace(&self, range: LockingRangeRef, column: u16) -> Result<ACERef, Error> {
        let acl = self.session.get_acl(range.as_uid(), method_id::SET).await?;
        for ace in acl {
            let columns: List<u16> = self.session.get(ace.as_uid(), ACE::COLUMNS).await?;
            if columns.0 == vec![column] {
                return Ok(ace);
            }
        }
        Err(Error::IncompatibleSSC)
    }

    async fn is_user(&self, authority: AuthorityRef) -> Result<bool, Error> {
        let class: AuthorityRef = self.session.get(authority.as_uid(), Authority::CLASS).await?;
        Ok(class == spec::opal::locking::authority::USERS) // The UID of Users is the same for all relevant SSCs.
    }
}

fn has_permission(expr: &[ACEOperand], authority: AuthorityRef) -> bool {
    eval_ace_expression(expr, &[authority]).unwrap_or(false)
}

fn eval_ace_expression(expr: &[ACEOperand], authenticated: &[AuthorityRef]) -> Option<bool> {
    let mut stack = Vec::<bool>::new();
    for item in expr {
        match item {
            ACEOperand::Authority(authority) => {
                if authenticated.contains(authority) {
                    stack.push(true);
                } else {
                    stack.push(false);
                }
            }
            ACEOperand::BooleanOp(BooleanOp::And) => {
                let rhs = stack.pop()?;
                let lhs = stack.pop()?;
                stack.push(lhs && rhs);
            }
            ACEOperand::BooleanOp(BooleanOp::Or) => {
                let rhs = stack.pop()?;
                let lhs = stack.pop()?;
                stack.push(lhs || rhs);
            }
            ACEOperand::BooleanOp(BooleanOp::Not) => {
                let arg = stack.pop()?;
                stack.push(!arg);
            }
        }
    }
    stack.first().cloned()
}

#[cfg(test)]
mod tests {
    use crate::{applications::utility::tests::setup_activated_tper, fake_device::MSID_PASSWORD, spec};

    use super::*;

    #[tokio::test]
    async fn list_users() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = PermissionEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let users = session.list_users().await?;
        assert_eq!(users.len(), 8);
        assert!(users.contains(&spec::opal::locking::authority::USER.nth(1).unwrap()));
        assert!(users.contains(&spec::opal::locking::authority::USER.nth(8).unwrap()));
        Ok(())
    }

    #[tokio::test]
    async fn list_ranges() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = PermissionEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let ranges = session.list_ranges().await?;
        assert_eq!(ranges.len(), 9);
        assert!(ranges.contains(&spec::opal::locking::locking::GLOBAL_RANGE));
        assert!(ranges.contains(&spec::opal::locking::locking::RANGE.nth(1).unwrap()));
        assert!(ranges.contains(&spec::opal::locking::locking::RANGE.nth(8).unwrap()));
        Ok(())
    }

    #[tokio::test]
    async fn mbr_permission() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = PermissionEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let user = spec::opal::locking::authority::USER.nth(1).unwrap();
        let mbr_permission = session.get_mbr_permission(user).await?;
        assert_eq!(mbr_permission, false);
        Ok(())
    }

    #[tokio::test]
    async fn read_permission() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = PermissionEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let user = spec::opal::locking::authority::USER.nth(1).unwrap();
        let range = spec::opal::locking::locking::GLOBAL_RANGE;
        let read_permission = session.get_read_permission(user, range).await?;
        assert_eq!(read_permission, false);
        Ok(())
    }

    #[tokio::test]
    async fn write_permission() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = PermissionEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let user = spec::opal::locking::authority::USER.nth(1).unwrap();
        let range = spec::opal::locking::locking::GLOBAL_RANGE;
        let write_permission = session.get_write_permission(user, range).await?;
        assert_eq!(write_permission, false);
        Ok(())
    }
}
