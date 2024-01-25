from niexpctrl_backend import Experiment as RawDLL
from .card import AOCardProxy, DOCardProxy
from typing import Optional, Literal


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

    def _add_card(
            self,
            card_type: Literal['AO', 'DO'],
            max_name: str,
            samp_rate: float,
            nickname: Optional[str] = None
    ):
        if card_type == 'AO':
            dll_method = RawDLL.add_ao_device
            proxy_class = AOCardProxy
            target_dict = self._ao_card_dict
        elif card_type == 'DO':
            dll_method = RawDLL.add_do_device
            proxy_class = DOCardProxy
            target_dict = self._do_card_dict
        else:
            raise ValueError(f'Invalid card type "{card_type}". Valid type strings are "AO" and "DO"')

        # Raw (maturin wrapped) DLL call
        dll_method(
            self._dll,
            max_name,  # FixMe[Rust]: change `physical_name` to `max_name`
            samp_rate=samp_rate
        )
        # Proxy object
        proxy = proxy_class(
            _dll=self._dll,
            max_name=max_name,
            nickname=nickname,
            samp_rate=samp_rate
        )
        target_dict[max_name] = proxy
        return proxy

    def add_ao_card(
            self,
            max_name: str,
            samp_rate: float,
            nickname: Optional[str] = None
    ):
        return self._add_card(
            card_type='AO',
            max_name=max_name,
            samp_rate=samp_rate,
            nickname=nickname
        )

    def add_do_card(
            self,
            max_name: str,
            samp_rate: float,
            nickname: Optional[str] = None
    ):
        return self._add_card(
            card_type='DO',
            max_name=max_name,
            samp_rate=samp_rate,
            nickname=nickname
        )

    def compile(self, stop_time: Optional[float] = None) -> float:
        return self._dll.compile(stop_time=stop_time)

    def stream_exp(
            self,
            stream_buftime: Optional[float] = 50,
            nreps: Optional[int] = 1
    ):
        self._dll.stream_exp(
            stream_buftime=stream_buftime,
            nreps=nreps
        )

    def add_reset_instr(self, reset_time: Optional[float] = None):
        self._dll.add_reset_instr(reset_time=reset_time)

    def clear_edit_cache(self):
        self._dll.clear_edit_cache()
        self._dll.clear_compile_cache()

    def check_trig_config(self):
        self._dll.check_trig_config()

    def reset_all(self):
        self._dll.reset_devices()
