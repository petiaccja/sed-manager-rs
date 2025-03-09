use crate::spec;
use crate::spec::objects::{MBRControl, TableDesc};
use crate::tper::{Session, TPer};

use super::{utility::start_admin1_session, Error};

pub struct MBREditSession {
    session: Session,
}

impl MBREditSession {
    pub async fn start(tper: &TPer, admin1_password: &[u8]) -> Result<Self, Error> {
        Ok(Self { session: start_admin1_session(tper, admin1_password).await? })
    }

    pub async fn end(self) -> Result<(), Error> {
        Ok(self.session.end_session().await?)
    }

    pub async fn get_size(&self) -> Result<u64, Error> {
        Ok(self.session.get(spec::core::table::MBR.as_uid(), TableDesc::ROWS).await?)
    }

    pub async fn set_enabled(&self, enabled: bool) -> Result<(), Error> {
        Ok(self.session.set(spec::core::mbr_control::MBR_CONTROL.as_uid(), MBRControl::ENABLE, enabled).await?)
    }

    pub async fn set_done(&self, done: bool) -> Result<(), Error> {
        Ok(self.session.set(spec::core::mbr_control::MBR_CONTROL.as_uid(), MBRControl::DONE, done).await?)
    }

    pub async fn get_enabled(&self) -> Result<bool, Error> {
        Ok(self.session.get(spec::core::mbr_control::MBR_CONTROL.as_uid(), MBRControl::ENABLE).await?)
    }

    pub async fn get_done(&self) -> Result<bool, Error> {
        Ok(self.session.get(spec::core::mbr_control::MBR_CONTROL.as_uid(), MBRControl::DONE).await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::applications::utility::tests::setup_activated_tper;
    use crate::fake_device::MSID_PASSWORD;
    use crate::messaging::discovery::LockingDescriptor;

    use super::*;

    async fn is_mbr_enabled(tper: &TPer) -> Result<bool, Error> {
        let discovery = tper.discover().await?;
        let locking_desc = discovery.get::<LockingDescriptor>().ok_or(Error::IncompatibleSSC)?;
        Ok(locking_desc.mbr_enabled)
    }

    async fn is_mbr_done(tper: &TPer) -> Result<bool, Error> {
        let discovery = tper.discover().await?;
        let locking_desc = discovery.get::<LockingDescriptor>().ok_or(Error::IncompatibleSSC)?;
        Ok(locking_desc.mbr_done)
    }

    #[tokio::test]
    async fn get_size() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = MBREditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let mbr_size = session.get_size().await?;
        assert_eq!(mbr_size, 0x08000000);
        Ok(())
    }

    #[tokio::test]
    async fn set_enabled() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = MBREditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        assert_eq!(is_mbr_enabled(&tper).await?, false);
        assert_eq!(session.get_enabled().await?, false);
        session.set_enabled(true).await?;
        assert_eq!(is_mbr_enabled(&tper).await?, true);
        assert_eq!(session.get_enabled().await?, true);
        Ok(())
    }

    #[tokio::test]
    async fn set_done() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = MBREditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        assert_eq!(is_mbr_done(&tper).await?, false);
        assert_eq!(session.get_done().await?, false);
        session.set_done(true).await?;
        assert_eq!(is_mbr_done(&tper).await?, true);
        assert_eq!(session.get_done().await?, true);
        Ok(())
    }
}
