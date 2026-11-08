//! Pure lifecycle state for the lazily-created selection icon.

#[derive(Default)]
pub struct OverlayLifecycle {
    generation: u64,
    epoch: u64,
    ready: bool,
    wanted: bool,
    capture_epoch: u64,
}

impl OverlayLifecycle {
    pub fn begin(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.ready = false;
        self.wanted = false;
        self.epoch = self.epoch.wrapping_add(1);
        self.generation
    }

    pub fn request_show(&mut self, capture_epoch: u64) -> bool {
        self.capture_epoch = capture_epoch;
        self.wanted = true;
        self.epoch = self.epoch.wrapping_add(1);
        self.ready
    }

    pub fn loaded(&mut self, generation: u64, latest_capture: u64) -> bool {
        if self.generation != generation { return false; }
        self.ready = true;
        let show = self.wanted && self.capture_epoch == latest_capture;
        if show { self.epoch = self.epoch.wrapping_add(1); }
        show
    }

    pub fn hide(&mut self) -> Option<(u64, u64)> {
        if !self.wanted { return None; }
        self.wanted = false;
        self.epoch = self.epoch.wrapping_add(1);
        Some((self.generation, self.epoch))
    }

    pub fn reclaimable(&self, generation: u64, epoch: u64) -> bool {
        self.generation == generation && self.epoch == epoch && (!self.wanted || !self.ready)
    }

    pub fn loading_token(&self) -> Option<(u64, u64)> {
        (!self.ready).then_some((self.generation, self.epoch))
    }

    pub fn closed(&mut self) { self.begin(); }
}

#[cfg(test)]
mod tests {
    use super::OverlayLifecycle;

    #[test]
    fn cold_icon_waits_for_loading_and_dismiss_during_load_stays_hidden() {
        let mut icon = OverlayLifecycle::default();
        let generation = icon.begin();
        assert!(!icon.request_show(1));
        icon.hide();
        assert!(!icon.loaded(generation, 2));
        assert!(icon.request_show(3));
    }

    #[test]
    fn late_load_cannot_show_an_outdated_selection() {
        let mut icon = OverlayLifecycle::default();
        let generation = icon.begin();
        icon.request_show(1);
        assert!(!icon.loaded(generation, 2));
        assert!(icon.request_show(2));
    }

    #[test]
    fn reuse_and_recreation_invalidate_old_reclaim_and_load_callbacks() {
        let mut icon = OverlayLifecycle::default();
        let first = icon.begin();
        icon.request_show(1);
        let (generation, epoch) = icon.hide().unwrap();
        assert!(icon.reclaimable(generation, epoch));
        assert!(icon.hide().is_none());
        icon.request_show(2);
        assert!(!icon.reclaimable(generation, epoch));
        let (generation, epoch) = icon.hide().unwrap();
        icon.closed();
        let second = icon.begin();
        icon.request_show(3);
        assert_ne!(first, second);
        assert!(!icon.loaded(first, 3));
        assert!(!icon.reclaimable(generation, epoch));
        assert!(icon.loaded(second, 3));
    }

    #[test]
    fn failed_load_can_be_reclaimed_but_a_ready_visible_icon_cannot() {
        let mut icon = OverlayLifecycle::default();
        let generation = icon.begin();
        icon.request_show(1);
        let (g, epoch) = icon.loading_token().unwrap();
        assert!(icon.reclaimable(g, epoch));
        assert!(icon.loaded(generation, 1));
        assert!(!icon.reclaimable(g, epoch));
        assert!(icon.loading_token().is_none());
    }
}
