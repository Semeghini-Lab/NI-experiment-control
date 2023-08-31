from niexpctrl_backend import Experiment as RawWrapClass
from typing import Optional, Literal, Union
import json


class NIStreamer:

    # ToDo: method to print entire device/channel tree

    class _BaseCard:
        def __init__(self, _exp: RawWrapClass, max_name: str):
            self._exp = _exp
            self.max_name = max_name
            self._chan_dict = {}

        def __getitem__(self, item):
            if item in self._chan_dict:
                return self._chan_dict[item]
            else:
                raise KeyError(f'There is no channel "{item}"')

        def __repr__(self):
            # ToDo: make a more informative message (card type, max_name, samp_rate, trig_mode, ...)
            return (
                f'{self.__class__}  card.\n'
                f'The following output channels have been added:\n'
                f'{json.dumps(self._chan_dict, indent=4)}'
            )

        def calc_signal(self, t_start, t_end, nsamps, require_streamable, require_editable):
            # ToDo: revisit
            self._exp.calc_signal(
                dev_name=self.max_name,  # FixMe[Rust]: change `dev_name` to `max_name`
                t_start=t_start,
                t_end=t_end,
                nsamps=nsamps,
                require_streamable=require_streamable,
                require_editable=require_editable,
            )

        def clear_edit_cache(self):
            self._exp.device_clear_edit_cache(dev_name=self.max_name)    # FixMe[Rust]: change `dev_name` to `max_name`

        def reset(self):
            self._exp.reset_device(dev_name=self.max_name)  # FixMe[Rust]: change `dev_name` to `max_name`

    class AOCard(_BaseCard):

        class OutChnl:
            # ToDo: implement __repr__()
            def __init__(self, _exp: RawWrapClass, _max_name: str, chan_idx: int):
                self._exp = _exp
                self._max_name = _max_name
                self.chan_idx = chan_idx

            def constant(self, t, val):
                # FixMe[Rust]: remove `duration` and `keep_val` arguments.
                #  @Nich: Is it possible to do in Rust? Or is it better to wrap it here? How?
                #  Details:
                #  having `keep_val` for const is redundand - it equivalent to setting `duration` to None.
                #  Using `duration` is also non-intuitive. What value should be kept after `duration`?
                #  Instead of using `duration` + `keep_val`,
                #  user would better just call `constant(t+duration, new_val)`

                raise NotImplementedError

                return self._exp.constant(
                    dev_name=self._max_name,  # FixMe[Rust]: change `dev_name` to `max_name`
                    chan_name=self.chan_idx,  # FixMe[Rust]: maybe change `chan_name` to `chan_idx`
                    t=t,
                    value=val  # FixMe[Rust]: change `value` to `val`
                )

            def sine(self, t, dur, amp, freq, phase=0, dc_offs=0, keep_val=False):
                # ToDo: try adding dur=None - when you just say "keep playing sine until further instructions"
                return self._exp.sine(
                    dev_name=self._max_name,
                    chan_name=f'ao{self.chan_idx}',
                    # FixMe[Rust]: here channel_id is expected to be str 'aoX', while everywhere else it is just int X.
                    #  Also, could we change `chan_name` to `chan_idx`? `chan_name` sounds like a string, while `chan_idx` - like an int
                    t=t,
                    duration=dur,
                    amplitude=amp,
                    freq=freq,
                    phase=phase if phase != 0 else None,  # FixMe[Rust]: better to use 0.0 instead of None for default. Is it conveninient in Rust?
                    dc_offset=dc_offs if dc_offs != 0 else None,  # FixMe[Rust]: better to use 0.0 instead of None for default
                    keep_val=keep_val,
                )

        def add_chnl(self, chan_idx: int):
            # Raw rust-maturin wrapper call
            self._exp.add_ao_channel(
                dev_name=self.max_name,  # FixMe[Rust]: change `dev_name` to `max_name`
                channel_id=chan_idx,  # FixMe[Rust]: maybe change `channel_id` to `chan_idx`
            )
            # Instantiate proxy object
            chnl_obj = self.OutChnl(
                _exp=self._exp,
                _max_name=self.max_name,
                chan_idx=chan_idx
            )
            self._chan_dict[chan_idx] = chnl_obj
            return chnl_obj

    class DOCard(_BaseCard):

        class OutChnl:
            # ToDo: implement __repr__()
            def __init__(self, _exp: RawWrapClass, _max_name: str, port_idx: int, line_idx: int):
                self._exp = _exp
                self._max_name = _max_name
                self.port_idx = port_idx
                self.line_idx = line_idx

            def go_high(self, t):
                return self._exp.go_high(
                    dev_name=self._max_name,
                    chan_name=f'port{self.port_idx}/line{self.line_idx}',
                    t=t
                )

            def go_low(self, t):
                return self._exp.go_low(
                    dev_name=self._max_name,
                    chan_name=f'port{self.port_idx}/line{self.line_idx}',
                    t=t
                )

        def add_chnl(self, port_idx: int, line_idx: int):
            # Raw rust-maturin wrapper call
            self._exp.add_do_channel(
                dev_name=self.max_name,  # FixMe[Rust]: change `dev_name` to `max_name`
                port_id=port_idx,  # FixMe[Rust]: maybe change `port_id` to `port_idx` - idx is associated with "int" - values from 0 to N-1, while "id" is more general
                line_id=line_idx  # FixMe[Rust]: maybe change `channel_id` to `chan_idx`
            )
            # Instantiate proxy object
            chnl_obj = self.OutChnl(
                _exp=self._exp,
                _max_name=self.max_name,
                port_idx=port_idx,
                line_idx=line_idx
            )
            self._chan_dict[f'port{port_idx}/line{line_idx}'] = chnl_obj
            return chnl_obj

    def __init__(self):
        self._exp = RawWrapClass()
        self._ao_card_dict = dict()
        self._do_card_dict = dict()

    def __getitem__(self, item):
        if item in self._ao_card_dict.keys():
            return self._ao_card_dict[item]
        elif item in self._do_card_dict.keys():
            return self._do_card_dict[item]
        else:
            raise KeyError(f'There is no card with max_name "{item}"')

    def __repr__(self):
        return (
            f'Experiment class.\n'
            f'The following AO cards have been added already:\n'
            f'{json.dumps(self._ao_card_dict, indent=4)}\n'
            f'\n'
            f'The following DO cards have been added already:\n'
            f'{json.dumps(self._do_card_dict, indent=4)}'
        )

    def add_card(
            self,
            card_type: Literal['AO', 'DO'],
            max_name: str,
            # Sample clock
            samp_rate: float,
            samp_clk_src: Optional[str] = None,
            # Start trigger
            trig_mode: Union[Literal['prim', 'sec'], None] = None,
            trig_line: Optional[str] = None,
            # Reference clock
            ref_clk_mode: Union[Literal['prim', 'sec'], None] = None,
            ref_clk_line: Optional[str] = None,
            ref_clk_rate: Optional[str] = None,
    ):
        if card_type == 'AO':
            raw_api_method = RawWrapClass.add_ao_device
            proxy_class = NIStreamer.AOCard
            target_dict = self._ao_card_dict
        elif card_type == 'DO':
            raw_api_method = RawWrapClass.add_do_device
            proxy_class = NIStreamer.DOCard
            target_dict = self._do_card_dict
        else:
            raise ValueError(f'Invalid card type "{card_type}". Valid type strings are "AO" and "DO"')

        if trig_mode not in ['prim', 'sec', None]:
            raise ValueError(f'Invalid trig_role "{trig_mode}". Valid values are "prim", "sec", and None')
        # FixMe[Rust]: Temporary fix while Rust side hasn't been adjusted
        if trig_mode == 'prim':
            trig_mode = True
        elif trig_mode == 'sec':
            trig_mode = False

        if ref_clk_mode not in ['prim', 'sec', None]:
            raise ValueError(f'Invalid ref_clk_role "{ref_clk_mode}". Valid values are "prim", "sec", and None')
        # FixMe[Rust]: Temporary fix while Rust side hasn't been adjusted
        if ref_clk_mode == 'prim':
            ref_clk_mode = True
        elif ref_clk_mode == 'sec':
            ref_clk_mode = False

        # Raw rust-maturin wrapper call
        raw_api_method(
            self._exp,
            physical_name=max_name,  # FixMe[Rust]: change `physical_name` to `max_name`
            # Sample clock
            samp_rate=samp_rate,
            samp_clk_src=samp_clk_src,
            # Start trigger
            is_primary=trig_mode,  # FixMe[Rust]: change `is_primary` to `trig_mode`
            trig_line=trig_line,
            # Reference clock
            import_ref_clk=ref_clk_mode,  # FixMe[Rust]: change `import_ref_clk` to `ref_clk_mode`
            ref_clk_line=ref_clk_line,
            ref_clk_rate=ref_clk_rate,
        )

        # Instantiate proxy object
        card_obj = proxy_class(_exp=self._exp, max_name=max_name)
        target_dict[max_name] = card_obj
        return card_obj

    def add_ao_card(self, *args, **kwargs):
        return self.add_card('AO', *args, **kwargs)

    def add_do_card(self, *args, **kwargs):
        return self.add_card('DO', *args, **kwargs)

    def compile(self, stop_time: Optional[float] = None) -> float:
        if stop_time is None:
            self._exp.compile()
        else:
            self._exp.compile_with_stoptime(stop_time=stop_time)

        return self._exp.compiled_stop_time()

    def stream_exp(
            self,
            stream_buftime: Optional[float] = 50,
            nreps: Optional[int] = 1
    ):
        self._exp.stream_exp(
            stream_buftime=stream_buftime,
            nreps=nreps
        )

    def clear_edit_cache(self):
        self._exp.clear_edit_cache()

    def reset_all(self):
        self._exp.reset_devices()
