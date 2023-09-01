from niexpctrl_backend import Experiment as RawDLL
from typing import Optional, Union, Literal
import card


class NIStreamer:

    def __init__(self):
        self._dll = RawDLL()
        self._ao_card_dict = dict()
        self._do_card_dict = dict()

    def __getitem__(self, item):
        if item in self._ao_card_dict.keys():
            return self._ao_card_dict[item]
        elif item in self._do_card_dict.keys():
            return self._do_card_dict[item]
        else:
            raise KeyError(f'There is no card with max_name "{item}"')

    # def __repr__(self):
    #     # FixMe: TypeError: Object of type AOCard is not JSON serializable
    #     return (
    #         f'Experiment class.\n'
    #         f'The following AO cards have been added already:\n'
    #         f'{json.dumps(self._ao_card_dict, indent=4)}\n'
    #         f'\n'
    #         f'The following DO cards have been added already:\n'
    #         f'{json.dumps(self._do_card_dict, indent=4)}'
    #     )

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
            raw_api_method = RawDLL.add_ao_device
            proxy_class = card.AOCard
            target_dict = self._ao_card_dict
        elif card_type == 'DO':
            raw_api_method = RawDLL.add_do_device
            proxy_class = card.DOCard
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
            self._dll,
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
        card_obj = proxy_class(_dll=self._dll, max_name=max_name)
        target_dict[max_name] = card_obj
        return card_obj

    def compile(self, stop_time: Optional[float] = None) -> float:
        if stop_time is None:
            self._dll.compile()
        else:
            self._dll.compile_with_stoptime(stop_time=stop_time)

        return self._dll.compiled_stop_time()

    def stream_exp(
            self,
            stream_buftime: Optional[float] = 50,
            nreps: Optional[int] = 1
    ):
        self._dll.stream_exp(
            stream_buftime=stream_buftime,
            nreps=nreps
        )

    def clear_edit_cache(self):
        self._dll.clear_edit_cache()
        self._dll.clear_compile_cache()

    def reset_all(self):
        self._dll.reset_devices()
