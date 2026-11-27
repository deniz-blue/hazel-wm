use smithay::{
    output::{Mode, Output},
    reexports::drm::control::Mode as DrmMode,
};

use super::HazelTtyBackend;
use crate::backend::ModeChange;

pub fn closest_mode(available: &[DrmMode], wanted: Mode) -> Option<DrmMode> {
    available
        .iter()
        .filter(|candidate| {
            let candidate = Mode::from(**candidate);
            candidate.size == wanted.size && (wanted.refresh == 0 || candidate.refresh == wanted.refresh)
        })
        .max_by_key(|candidate| Mode::from(**candidate).refresh)
        .copied()
}

impl HazelTtyBackend {
    pub fn use_mode(&mut self, output: &Output, wanted: Mode) -> Result<ModeChange, String> {
        let surface = self
            .output_devices
            .values_mut()
            .flat_map(|device| device.surfaces.values_mut())
            .find(|surface| surface.output == *output)
            .ok_or_else(|| format!("{} is not driven by this backend", output.name()))?;
        let drm_mode = closest_mode(&surface.drm_modes, wanted)
            .ok_or_else(|| format!("{} has no mode {}x{}", output.name(), wanted.size.w, wanted.size.h))?;
        surface.compositor.use_mode(drm_mode).map_err(|e| format!("{e:?}"))?;
        output.change_current_state(Some(Mode::from(drm_mode)), None, None, None);
        Ok(ModeChange::Applied)
    }
}
