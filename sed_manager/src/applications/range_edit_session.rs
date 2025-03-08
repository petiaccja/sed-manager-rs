use crate::spec::column_types::{CredentialRef, LockingRangeRef, MediaKeyRef};
use crate::spec::objects::LockingRange;
use crate::spec::table_id;
use crate::tper::{Session, TPer};

use super::utility::start_admin1_session;
use super::Error;

pub struct RangeEditSession {
    session: Session,
}

impl RangeEditSession {
    pub async fn start(tper: &TPer, admin1_password: &[u8]) -> Result<Self, Error> {
        Ok(Self { session: start_admin1_session(tper, admin1_password).await? })
    }

    pub async fn end(self) -> Result<(), Error> {
        Ok(self.session.end_session().await?)
    }

    pub async fn list_ranges(&self) -> Result<Vec<LockingRangeRef>, Error> {
        let ranges = self.session.next(table_id::LOCKING, None, None).await?;
        Ok(ranges.into_iter().filter_map(|uid| LockingRangeRef::try_from(uid).ok()).collect())
    }

    pub async fn get_range(&self, range: LockingRangeRef) -> Result<LockingRange, Error> {
        let columns = LockingRange::RANGE_START..=LockingRange::WRITE_LOCKED;
        let (range_start, range_length, read_lock_enabled, write_lock_enabled, read_locked, write_locked) =
            self.session.get_multiple::<(u64, u64, bool, bool, bool, bool)>(range.as_uid(), columns).await?;

        Ok(LockingRange {
            uid: range,
            range_start,
            range_length,
            read_lock_enabled,
            write_lock_enabled,
            read_locked,
            write_locked,
            ..Default::default()
        })
    }

    pub async fn set_range(&self, range: &LockingRange) -> Result<(), Error> {
        let is_global_range = range.uid == crate::spec::opal::locking::locking::GLOBAL_RANGE;
        if !is_global_range {
            let columns: [u16; 6] = core::array::from_fn(|i| LockingRange::RANGE_START + (i as u16));
            let values = (
                range.range_start,
                range.range_length,
                range.read_lock_enabled,
                range.write_lock_enabled,
                range.read_locked,
                range.write_locked,
            );
            Ok(self.session.set_multiple(range.uid.as_uid(), columns, values).await?)
        } else {
            let columns: [u16; 4] = core::array::from_fn(|i| LockingRange::READ_LOCK_ENABLED + (i as u16));
            let values = (range.read_lock_enabled, range.write_lock_enabled, range.read_locked, range.write_locked);
            Ok(self.session.set_multiple(range.uid.as_uid(), columns, values).await?)
        }
    }

    pub async fn erase_range(&self, range: LockingRangeRef) -> Result<(), Error> {
        let active_key_id: MediaKeyRef = self.session.get(range.as_uid(), LockingRange::ACTIVE_KEY).await?;
        Ok(self.session.gen_key(CredentialRef::new_other(active_key_id), None, None).await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::applications::utility::tests::setup_activated_tper;
    use crate::fake_device::MSID_PASSWORD;
    use crate::spec;

    use super::*;

    #[tokio::test]
    async fn list_ranges() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = RangeEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let ranges = session.list_ranges().await?;
        assert_eq!(ranges.len(), 9);
        assert!(ranges.contains(&spec::opal::locking::locking::GLOBAL_RANGE));
        assert!(ranges.contains(&spec::opal::locking::locking::RANGE.nth(1).unwrap()));
        assert!(ranges.contains(&spec::opal::locking::locking::RANGE.nth(8).unwrap()));
        Ok(())
    }

    #[tokio::test]
    async fn set_get_global_range() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = RangeEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let uid = spec::opal::locking::locking::GLOBAL_RANGE;
        let range = session.get_range(uid).await?;
        assert_eq!(range.uid, uid);
        let modified = LockingRange { range_length: 10000, read_lock_enabled: true, write_locked: true, ..range };
        session.set_range(&modified).await?;
        let expected = LockingRange { range_length: 0, ..modified };
        let range = session.get_range(uid).await?;
        assert_eq!(range, expected);
        Ok(())
    }

    #[tokio::test]
    async fn set_get_any_range() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = RangeEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let uid = spec::opal::locking::locking::RANGE.nth(1).unwrap();
        let range = session.get_range(uid).await?;
        assert_eq!(range.uid, uid);
        let modified = LockingRange { range_length: 10000, read_lock_enabled: true, write_locked: true, ..range };
        session.set_range(&modified).await?;
        let expected = modified;
        let range = session.get_range(uid).await?;
        assert_eq!(range, expected);
        Ok(())
    }

    #[tokio::test]
    async fn erase_range() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = RangeEditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let uid = spec::opal::locking::locking::GLOBAL_RANGE;
        session.erase_range(uid).await?;
        // We cannot really check if GenKey was called properly, because the FakeDevice does
        // not implement GenKey meaningfully. Let's just see if it fails, as FakeDevice does
        // check for incorrect UIDs.
        Ok(())
    }
}
