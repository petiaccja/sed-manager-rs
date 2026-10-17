//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_packet::{
    MaxBytes,
    discovery::{FeatureDescriptor, LockingDescriptor},
};
use sed_spec::{
    methods::MethodStatus,
    objects::{
        Authority, AuthorityRef, LockingInfoExt, LockingRange, LockingRangeRef, LockingRangeRefExt, MbrControl,
        TableDescRefExt,
    },
    preconfig::core::shared::{locking_info, mbr_control, table, table_id},
};
use sed_tper::{Error as TperError, Session, Tper};
use tracing::{debug, instrument};

use crate::{
    Alignment, LockingRangePlanError,
    authority_plan::{AuthorityPlanError, AuthorityPlanner},
    error::Error,
    locking_range_plan::{LockingRangePlanner, LockingRangeStep},
    set_plan::check_matching_current_all,
    spec::Spec,
};

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

    /// Get the locking range alignment requirements.
    ///
    /// If the device does not specify the alignment requirements (e.g.
    /// Enterprise, Opal 1.x), or the device only partially specifies the
    /// requiements (not actually allowed by spec), the missing fields are
    /// defaulted (See [`Alignment`]).
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn get_alignment(&self) -> Result<Alignment, Error> {
        let maybe_locking_info: Result<LockingInfoExt, _> =
            self.session.get_object(locking_info::LOCKING_INFO, 7..11).await;
        match maybe_locking_info {
            Ok(LockingInfoExt { alignmnet_required, alignment_granularity, lowest_aligned_lba, .. }) => Ok(Alignment {
                alignment_required: alignmnet_required.unwrap_or(Alignment::default().alignment_required),
                alignment_granularity: alignment_granularity
                    .unwrap_or(Alignment::default().alignment_granularity)
                    .max(1),
                lowest_aligned_lba: lowest_aligned_lba.unwrap_or(Alignment::default().lowest_aligned_lba),
            }),
            Err(sed_tper::Error::MethodCallFailed(MethodStatus::InvalidParameter)) => Ok(Alignment::default()),
            Err(err) => Err(err.into()),
        }
    }

    /// Set the locking ranges to their `desired` value.
    ///
    /// When `expected` is provided, this function performs a compare-and-swap.
    /// If the current state of the locking ranges, as retrieved from the device,
    /// matches that given by `expected`, the ranges are updated to `desired`.
    /// If there is a mismatch, the ranges aren't updated and an error is returned.
    ///
    /// Only the fields of the [`LockingRange`] object of `desired` that are set
    /// to [`Some`] value are changed. Except for the UID, ranges should not
    /// attempt to set read-only or unauthorized fields, or else the device will
    /// reject the request and an error will be returned.
    ///
    /// Before writing anything to the device, the `desired` ranges are checked
    /// and rejected early if the device would reject them anyway. This avoids
    /// partial writes to the device, but the update procedure may still fail
    /// in the middle, leaving some ranges updated and other not. In either cases,
    /// an error is returned.
    #[instrument(skip(self), err)]
    pub async fn set_locking_ranges(
        &self,
        expected: Option<&[LockingRange]>,
        desired: &[LockingRange],
        capacity: u64,
    ) -> Result<(), Error> {
        let current = self.get_locking_ranges().await?;
        if let Some(expected) = expected {
            check_matching_current_all(expected, &current).map_err(LockingRangePlanError::from)?;
        }

        let alignment = self.get_alignment().await?;
        let planner = LockingRangePlanner::new(current, desired.iter().cloned())?;
        planner.check_alignment(&alignment)?;
        planner.check_within_capacity(capacity)?;

        let plan = planner.plan();
        debug!(plan = ?plan);
        for step in plan {
            match step {
                LockingRangeStep::Collapse(uid) => self.collapse_locking_range(uid).await,
                LockingRangeStep::Set { uid, changes } => self.set_locking_range(uid, changes).await,
            }
            .map_err(Error::LockingRangeUpdateFailed)?;
        }

        Ok(())
    }

    /// Erase the locking range.
    ///
    /// Uses the `GenKey` or `Erase` (Enterprise SSC) functions to discard the
    /// key associated with the locking range. On Enterprise, `Erase` resets the
    /// associated `BandMaster` authority as well.
    ///
    /// # Errors
    ///
    /// Besides the usual, [`Error::NotSupported`] is returned if the device
    /// does not use media encryption (e.g. Pyrite), as there is no key to
    /// discard.
    ///
    /// # Notes
    ///
    /// Currently, Enterprise is not implemented and an error is always returned.
    #[instrument(skip(self), err)]
    pub async fn erase_locking_range(&self, locking_range: LockingRangeRef) -> Result<(), Error> {
        let locking_descriptor = self.spec.discovery.get::<LockingDescriptor>();
        let ssc_descriptor = self.spec.default_ssc_descriptor();

        if !locking_descriptor.is_some_and(|desc| desc.media_encryption) {
            Err(Error::NotSupported)
        } else if matches!(ssc_descriptor, Some(FeatureDescriptor::Enterprise(_))) {
            Err(Error::NotImplemented)
        } else {
            let key = self.session.get_field(locking_range.active_key()).await?;
            self.session.gen_key(key, None, None).await?;
            Ok(())
        }
    }

    /// Set the authorities to their `desired` value.
    ///
    /// Like [`set_locking_ranges`], it performs a compare-and-swap, and only
    /// changes the authorities on the devise if `expected` matches the current
    /// state of the authorities.
    #[instrument(skip(self), err)]
    pub async fn set_authorities(&self, expected: Option<&[Authority]>, desired: &[Authority]) -> Result<(), Error> {
        let current = self.get_authorities().await?;
        if let Some(expected) = expected {
            check_matching_current_all(expected, &current).map_err(AuthorityPlanError::from)?;
        }

        let planner = AuthorityPlanner::new(current, desired.iter().cloned())?;

        let plan = planner.plan();
        debug!(plan = ?plan);
        for step in plan {
            let changes = Authority { uid: None, ..step.changes };
            self.session.set_object(step.uid, changes).await.map_err(Error::AuthorityUpdateFailed)?;
        }

        Ok(())
    }

    #[instrument(skip(self), err)]
    async fn collapse_locking_range(&self, uid: LockingRangeRef) -> Result<(), TperError> {
        let changes = LockingRange { range_length: Some(0), ..Default::default() };
        self.set_locking_range(uid, changes).await
    }

    #[instrument(skip(self), err)]
    async fn set_locking_range(&self, uid: LockingRangeRef, changes: LockingRange) -> Result<(), TperError> {
        let changes = LockingRange { uid: None, ..changes };
        self.session.set_object(uid, changes).await
    }
}
