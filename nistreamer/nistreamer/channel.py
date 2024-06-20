from niexpctrl_backend import Experiment as RawStreamer  # FixMe[Rust]: rename Experiment to NIStreamer


class BaseChanProxy:
    def __init__(
            self,
            _streamer: RawStreamer,
            _card_max_name: str,
            nickname: str = None
    ):
        self._streamer = _streamer
        self._card_max_name = _card_max_name
        self._nickname = nickname

    def __repr__(self, card_info=False):
        return (
            f'Channel {self.chan_name} on card {self._card_max_name}\n'
            f'Default value: {self.default_val}'
        )

    @property
    def chan_name(self):
        raise NotImplementedError

    @property
    def default_val(self):
        return self._streamer.chan_get_default_val(
            dev_name=self._card_max_name,
            chan_name=self.chan_name
        )

    @property
    def nickname(self):
        if self._nickname is not None:
            return self._nickname
        else:
            return self.chan_name

    def clear_edit_cache(self):
        self._streamer.channel_clear_edit_cache(
            dev_name=self._card_max_name,
            chan_name=self.chan_name
        )
        self._streamer.channel_clear_compile_cache(
            dev_name=self._card_max_name,
            chan_name=self.chan_name
        )

    def calc_signal(self, t_start=None, t_end=None, nsamps=1000):

        # FixMe[Rust]: panic message `PanicException: Attempting to calculate signal on not-compiled channel ao0`
        #  - add card_max_name to know which card this is about

        # ToDo: figure out details of edit_cache/compile_cache.
        #  `self._dll.is_fresh_compiled()` may still give True even after introducing changes???
        # if not self._dll.is_fresh_compiled():
        #     self._dll.compile()
        # ToDo: until then go inefficient but safe - recompile from scratch every time
        # self._dll.compile()

        t_start = t_start if t_start is not None else 0.0
        # FixMe: if channel was compiled with some `stop_time`,
        #  using `last_instr_end_time()` will "truncate" the padding tail.
        #  Ideally, one would rather use `BaseChannel::total_run_time()` but it is not exposed now.
        #  Either expose it or consider changing the signature of the underlying `BaseChannel::calc_signal_nsamps()`
        #  to accept `Option<start_time>` and `Option<end_time>`
        t_end = t_end if t_end is not None else self.last_instr_end_time()

        signal_arr = self._streamer.channel_calc_signal_nsamps(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            start_time=t_start,  # FixMe[Rust]: unify `start_time` and `t_start`
            end_time=t_end,
            num_samps=nsamps,  # FixMe[Rust]: unify `num_samps` and `nsamps`
        )

        return t_start, t_end, signal_arr

    def last_instr_end_time(self):
        return self._streamer.channel_last_instr_end_time(
            dev_name=self._card_max_name,
            chan_name=self.chan_name
        )


class AOChanProxy(BaseChanProxy):
    def __init__(
            self,
            _streamer: RawStreamer,
            _card_max_name: str,
            chan_idx: int,
            nickname: str = None
    ):
        # ToDo[Tutorial]: pass through all arguments to parent's __init__, maybe with *args, **kwargs,
        #  but such that argument completion hints are still coming through.

        BaseChanProxy.__init__(
            self,
            _streamer=_streamer,
            _card_max_name=_card_max_name,
            nickname=nickname
        )
        self.chan_idx = chan_idx

    @property
    def chan_name(self):
        return f'ao{self.chan_idx}'

    def constant(self, t, dur, val):
        self._streamer.constant(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t,
            duration=dur,
            value=val,
        )
        return dur
    
    def go_constant(self, t, val):
        self._streamer.go_constant(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t,
            value=val,
        )

    def sine(self, t, dur, amp, freq, phase=0, dc_offs=0, keep_val=False):
        self._streamer.sine(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t,
            duration=dur,
            amplitude=amp,
            freq=freq,
            phase=phase if phase != 0 else None,
            # FixMe[Rust]: better to use 0.0 instead of None for default. Is it conveninient in Rust?
            dc_offset=dc_offs if dc_offs != 0 else None,  # FixMe[Rust]: better to use 0.0 instead of None for default
            keep_val=keep_val,
        )
        return dur
    
    def go_sine(self, t, amp, freq, phase=0, dc_offs=0):
        self._streamer.go_sine(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t,
            amplitude=amp,
            freq=freq,
            phase=phase if phase != 0 else None,
            # FixMe[Rust]: better to use 0.0 instead of None for default. Is it conveninient in Rust?
            dc_offset=dc_offs if dc_offs != 0 else None,  # FixMe[Rust]: better to use 0.0 instead of None for default
        )

    def linramp(self, t, dur, start_val, end_val, keep_val=True):
        self._streamer.linramp(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t,
            duration=dur,
            start_val=start_val, 
            end_val=end_val,
            keep_val=keep_val,
        )
        return dur


class DOChanProxy(BaseChanProxy):
    def __init__(
            self,
            _streamer: RawStreamer,
            _card_max_name: str,
            port_idx: int,
            line_idx: int,
            nickname: str = None
    ):
        BaseChanProxy.__init__(
            self,
            _streamer=_streamer,
            _card_max_name=_card_max_name,
            nickname=nickname
        )
        self.port_idx = port_idx
        self.line_idx = line_idx

    @property
    def chan_name(self):
        return f'port{self.port_idx}/line{self.line_idx}'

    @property
    def default_val(self):
        float_val = self._streamer.chan_get_default_val(
            dev_name=self._card_max_name,
            chan_name=self.chan_name
        )
        # ToDo: remove this hack when AO/DO types are split in Rust backend
        return True if float_val > 0.5 else False

    def go_high(self, t):
        self._streamer.go_high(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t
        )

    def go_low(self, t):
        self._streamer.go_low(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t
        )

    def high(self, t, dur):
        self._streamer.high(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t,
            duration=dur
        )
        return dur

    def low(self, t, dur):
        self._streamer.low(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t,
            duration=dur
        )
        return dur
