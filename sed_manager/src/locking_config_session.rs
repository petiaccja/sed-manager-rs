//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_packet::MaxBytes;
use sed_spec::{
    objects::{Authority, AuthorityRef, LockingRange, MbrControl, TableDescRefExt},
    preconfig::core::shared::{mbr_control, table, table_id},
};
use sed_tper::{Session, Tper};
use tracing::instrument;

use crate::{error::Error, spec::Spec};

/// Configures the locking SP of the TPer, like locking ranges and authorities.
///
/// This is a persistent session and holds a [`Session`] object for the
/// lifetime of the object. Requests to configure the locking SP are performed
/// within that session.
#[derive(Debug)]
pub struct LockingConfigSession {
    spec: Spec,
    session: Session,
}

impl LockingConfigSession {
    /// Start the configuration session on the locking SP as `authority`. The
    /// permissions of the authority determine which actions can be performed.
    ///
    /// For devices that support multiple SSCs, the chosen SSC is identified
    /// by the `spec`.
    #[instrument(level = "info", skip(tper, password), ret, err)]
    pub async fn login(
        tper: &Tper,
        spec: Spec,
        authority: AuthorityRef,
        password: Option<MaxBytes<32>>,
    ) -> Result<Self, Error> {
        let locking_sp_uid = spec.locking.as_ref().map(|sp| sp.uid).ok_or(Error::IncompatibleSsc)?;
        let session = tper.start_session(locking_sp_uid, Some(authority), password).await?;
        Ok(Self { spec, session })
    }

    /// Start the configuration session on the locking SP as `authority`. The
    /// permissions of the authority determine which actions can be performed.
    ///
    /// For devices that support multiple SSCs, the primary SSC is chosen. See
    /// [`Spec`] about how the primary SSC is chosen.
    #[instrument(level = "info", skip(tper, password), ret, err)]
    pub async fn login_on_primary_ssc(
        tper: &Tper,
        authority: AuthorityRef,
        password: Option<MaxBytes<32>>,
    ) -> Result<Self, Error> {
        let discovery = tper.discover_current().await?;
        let spec = Spec::try_from(discovery).map_err(|_| Error::NoSscAvailable)?;
        Self::login(tper, spec, authority, password).await
    }

    /// Return the [`Spec`] this session is using.
    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    /// Close the internal session to the SP. See [`Session::close`] for how
    /// [`Drop`] is handled.
    pub async fn close(self) -> Result<(), Error> {
        self.session.close().await.map_err(|err| err.into())
    }

    /// Get the list of authorities and their columns.
    ///
    /// This function will attempt to retrieve all columns of the authorities.
    /// The columns returned may vary based on which authority is authenticated
    /// in this session.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn get_authorities(&self) -> Result<Vec<Authority>, Error> {
        let authority_refs = self.session.next::<{ table_id::AUTHORITY.to_u64() }>(None, None).await?;
        let mut authorities = Vec::new();
        for authority_ref in authority_refs {
            authorities.push(self.session.get_object(authority_ref, ..).await?);
        }
        Ok(authorities)
    }

    /// Get the list of locking ranges and their columns.
    ///
    /// This function will attempt to retrieve all columns of the locking ranges.
    /// The columns returned may vary based on which authority is authenticated
    /// in this session.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn get_locking_ranges(&self) -> Result<Vec<LockingRange>, Error> {
        let range_refs = self.session.next::<{ table_id::LOCKING.to_u64() }>(None, None).await?;
        let mut ranges = Vec::new();
        for range_ref in range_refs {
            ranges.push(self.session.get_object(range_ref, ..).await?);
        }
        Ok(ranges)
    }

    /// Get MBR table size.
    ///
    /// # Errors
    ///
    /// Besides the usual, an invalid parameter error is returned if the MBR is
    /// not supported. (In this case, the Table table has no entry for the MBR
    /// table.)
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn get_mbr_size(&self) -> Result<u32, Error> {
        self.session.get_field(table::MBR.rows()).await.map_err(|err| err.into())
    }

    /// Get the MBR control parameters.
    ///
    /// Some columns of the MBR object may not be returned if the authenticated
    /// authority has no rights to read them.
    ///
    /// # Errors
    ///
    /// Besides the usual, an invalid parameter error is returned if the MBR is
    /// not supported. (In this case, the MBRControl table is missing.)
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn get_mbr_control(&self) -> Result<MbrControl, Error> {
        self.session.get_object(mbr_control::MBR_CONTROL, ..).await.map_err(|err| err.into())
    }
}
