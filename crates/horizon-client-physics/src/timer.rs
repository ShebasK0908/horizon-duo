//! `DeltaTracker.Timer` and the tick-rate managers of 26.3: how real
//! milliseconds turn into game ticks, partial ticks and real-time deltas, and
//! the `/tick` freeze, step and sprint state.

/// `DeltaTracker.Timer`: float arithmetic on integer milliseconds, exactly as
/// the client does it.
#[derive(Clone, Debug)]
pub struct Timer {
    delta_ticks: f32,
    delta_tick_residual: f32,
    realtime_delta_ticks: f32,
    paused_delta_tick_residual: f32,
    last_ms: i64,
    last_ui_ms: i64,
    ms_per_tick: f32,
    paused: bool,
    frozen: bool,
}

impl Timer {
    pub fn new(ticks_per_second: f32, current_ms: i64) -> Self {
        Self {
            delta_ticks: 0.0,
            delta_tick_residual: 0.0,
            realtime_delta_ticks: 0.0,
            paused_delta_tick_residual: 0.0,
            last_ms: current_ms,
            last_ui_ms: current_ms,
            ms_per_tick: 1000.0 / ticks_per_second,
            paused: false,
            frozen: false,
        }
    }

    /// `advanceGameTime`: the whole ticks to run now. `target_mspt` maps the
    /// default milliseconds per tick to the target in force (the tick-rate
    /// manager's, when the level runs normally). The caller runs at most ten.
    pub fn advance_game_time(
        &mut self,
        current_ms: i64,
        target_mspt: impl FnOnce(f32) -> f32,
    ) -> i32 {
        self.delta_ticks = (current_ms - self.last_ms) as f32 / target_mspt(self.ms_per_tick);
        self.last_ms = current_ms;
        self.delta_tick_residual += self.delta_ticks;
        let ticks = self.delta_tick_residual as i32;
        self.delta_tick_residual -= ticks as f32;
        ticks
    }

    /// `advanceRealTime`.
    pub fn advance_real_time(&mut self, current_ms: i64) {
        self.realtime_delta_ticks = (current_ms - self.last_ui_ms) as f32 / self.ms_per_tick;
        self.last_ui_ms = current_ms;
    }

    /// `updatePauseState`.
    pub fn update_pause_state(&mut self, pause: bool) {
        if pause {
            if !self.paused {
                self.paused_delta_tick_residual = self.delta_tick_residual;
            }
            self.paused = true;
        } else {
            if self.paused {
                self.delta_tick_residual = self.paused_delta_tick_residual;
            }
            self.paused = false;
        }
    }

    /// `updateFrozenState`.
    pub fn update_frozen_state(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    pub fn game_time_delta_ticks(&self) -> f32 {
        self.delta_ticks
    }

    /// `getGameTimeDeltaPartialTick`.
    pub fn game_time_delta_partial_tick(&self, ignore_frozen_game: bool) -> f32 {
        if !ignore_frozen_game && self.frozen {
            1.0
        } else if self.paused {
            self.paused_delta_tick_residual
        } else {
            self.delta_tick_residual
        }
    }

    /// `getRealtimeDeltaTicks`.
    pub fn realtime_delta_ticks(&self) -> f32 {
        if self.realtime_delta_ticks > 7.0 {
            0.5
        } else {
            self.realtime_delta_ticks
        }
    }
}
