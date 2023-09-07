from niexpctrl_backend import Experiment as RawDLL
# FixMe[Rust]: rename Experiment to NIStreamer


class BaseChanProxy:
    def __init__(
            self,
            _dll: RawDLL,
            _card_max_name: str,
            nickname: str = None
    ):
        self._dll = _dll
        self._card_max_name = _card_max_name
        self._nickname = nickname

    def __repr__(self, card_info=False):
        return f'Channel {self.chan_name} on card {self._card_max_name}'

    @property
    def chan_name(self):
        raise NotImplementedError

    @property
    def nickname(self):
        if self._nickname is not None:
            return self._nickname
        else:
            return self.chan_name

    def clear_edit_cache(self):
        self._dll.channel_clear_edit_cache(
            dev_name=self._card_max_name,
            chan_name=self.chan_name
        )
        self._dll.channel_clear_compile_cache(
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
        self._dll.compile()

        t_start = t_start if t_start is not None else 0.0
        t_end = t_end if t_end is not None else self._dll.compiled_stop_time()

        signal_arr = self._dll.channel_calc_signal_nsamps(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            start_time=t_start,  # FixMe[Rust]: unify `start_time` and `t_start`
            end_time=t_end,
            num_samps=nsamps,  # FixMe[Rust]: unify `num_samps` and `nsamps`
        )

        return t_start, t_end, signal_arr


class AOChanProxy(BaseChanProxy):
    def __init__(
            self,
            _dll: RawDLL,
            _card_max_name: str,
            chan_idx: int,
            nickname: str = None
    ):
        # ToDo[Tutorial]: pass through all arguments to parent's __init__, maybe with *args, **kwargs,
        #  but such that argument completion hints are still coming through.

        BaseChanProxy.__init__(
            self,
            _dll=_dll,
            _card_max_name=_card_max_name,
            nickname=nickname
        )
        self.chan_idx = chan_idx

    @property
    def chan_name(self):
        return f'ao{self.chan_idx}'

    def constant(self, t, dur, val, keep_val=False):
        # FixMe[Rust]: remove `duration` and `keep_val` arguments.
        #  @Nich: Is it possible to do in Rust? Or is it better to wrap it here? How?
        #  Details:
        #  having `keep_val` for const is redundand - it equivalent to setting `duration` to None.
        #  Using `duration` is also non-intuitive. What value should be kept after `duration`?
        #  Instead of using `duration` + `keep_val`,
        #  user would better just call `constant(t+duration, new_val)`

        self._dll.constant(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t,
            duration=dur,
            value=val,
            keep_val=keep_val
        )

        return dur

        # raise NotImplementedError
        #
        # return self._dll.constant(
        #     dev_name=self._card_max_name,  # FixMe[Rust]: change `dev_name` to `max_name`
        #     chan_name=self.chan_name,
        #     t=t,
        #     value=val  # FixMe[Rust]: change `value` to `val`
        # )

    def sine(self, t, dur, amp, freq, phase=0, dc_offs=0, keep_val=False):
        # ToDo: try adding dur=None - when you just say "keep playing sine until further instructions"
        self._dll.sine(
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
        return t + dur


class DOChanProxy(BaseChanProxy):
    def __init__(
            self,
            _dll: RawDLL,
            _card_max_name: str,
            port_idx: int,
            line_idx: int,
            nickname: str = None
    ):
        BaseChanProxy.__init__(
            self,
            _dll=_dll,
            _card_max_name=_card_max_name,
            nickname=nickname
        )

        self.port_idx = port_idx
        self.line_idx = line_idx

    @property
    def chan_name(self):
        return f'port{self.port_idx}/line{self.line_idx}'

    def go_high(self, t):
        self._dll.go_high(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t
        )

    def go_low(self, t):
        self._dll.go_low(
            dev_name=self._card_max_name,
            chan_name=self.chan_name,
            t=t
        )
