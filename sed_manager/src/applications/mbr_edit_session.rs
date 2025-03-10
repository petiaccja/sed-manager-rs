use crate::messaging::packet::{PACKET_HEADER_LEN, SUB_PACKET_HEADER_LEN};
use crate::rpc::Properties;
use crate::spec;
use crate::spec::objects::{MBRControl, TableDesc};
use crate::spec::table_id;
use crate::tper::{Session, TPer};

use super::{utility::start_admin1_session, Error};

pub struct MBREditSession {
    session: Session,
    properties: Properties,
}

impl MBREditSession {
    pub async fn start(tper: &TPer, admin1_password: &[u8]) -> Result<Self, Error> {
        let properties = tper.current_properties().await;
        Ok(Self { session: start_admin1_session(tper, admin1_password).await?, properties })
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

    /// Upload data to the MBR table.
    ///
    /// The argument to [`Self::upload`] could be some AsyncRead trait, but those require `std`.
    /// Instead, the arguments are some simple functions. None of them should be blocking.
    ///
    /// * `read`: Reads the next chunk of data into its \[u8] buffer argument. Similar to std::io::Read.
    /// * `progress`: Periodically called with the number of bytes received.
    /// * `cancelled`: Periodically called an should return true to request a cancel.
    pub async fn upload(
        &self,
        mut read: impl AsyncFnMut(&mut [u8]) -> Result<usize, Error>,
        mut progress: impl FnMut(u64),
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<(), Error> {
        const CALL_LEN: usize = 128; // An upper bound for the encoding of the Set call that wraps the data token.
        let chunk_len = core::cmp::min(
            self.properties.max_gross_packet_size - PACKET_HEADER_LEN - SUB_PACKET_HEADER_LEN - CALL_LEN,
            self.properties.max_ind_token_size - 4,
        );
        let mut chunk = vec![0; chunk_len];
        let mut position: u64 = 0;
        while !cancelled() {
            let read_result = read(chunk.as_mut_slice()).await;
            let read_chunk_len = match read_result {
                Ok(0) => return Ok(()),
                Ok(n) => n,
                Err(err) => return Err(err),
            };
            let read_chunk = &chunk[0..read_chunk_len];
            self.session.write(table_id::MBR, position, read_chunk).await?;
            position += read_chunk_len as u64;
            progress(position);
        }
        Err(Error::Cancelled)
    }
}

#[cfg(test)]
mod tests {
    use std::u64;

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

    fn make_simulated_file(size: u64) -> impl AsyncFnMut(&mut [u8]) -> Result<usize, Error> {
        let mut cursor: u64 = 0;
        async move |out: &mut [u8]| -> Result<usize, Error> {
            let start = cursor;
            let end = core::cmp::min(start + out.len() as u64, size);
            cursor = end;
            let written_slice = &mut out[0..(end - start) as usize];
            let values = (0..u64::MAX).into_iter();
            core::iter::zip(written_slice.iter_mut(), values).for_each(|(item, value)| *item = value as u8);
            Ok((end - start) as usize)
        }
    }

    #[tokio::test]
    async fn test_simulated_file() -> Result<(), Error> {
        let mut chunk = vec![0; 1024];
        let mut file = make_simulated_file(2356);
        assert_eq!(1024, file(chunk.as_mut_slice()).await?);
        assert_eq!(1024, file(chunk.as_mut_slice()).await?);
        assert_eq!(308, file(chunk.as_mut_slice()).await?);
        assert_eq!(0, file(chunk.as_mut_slice()).await?);
        Ok(())
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

    #[tokio::test]
    async fn upload_success() -> Result<(), Error> {
        let tper = setup_activated_tper().await;
        let session = MBREditSession::start(&tper, MSID_PASSWORD.as_bytes()).await?;
        let file = make_simulated_file(1 * 1024 * 1024); // 1 megabyte
        session.upload(file, |_| (), || false).await
    }
}
