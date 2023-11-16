from niexpctrl_backend import Experiment as RawDLL
from .channel import AOChanProxy, DOChanProxy
from typing import Optional, Union, Literal


class BaseCardProxy:

    def __init__(
            self,
            _dll: RawDLL,
            max_name: str,
            samp_rate: float,
            nickname=None
    ):
        self._dll = _dll
        self.max_name = max_name
        self._nickname = nickname

        # Sample clock
        self.samp_rate = samp_rate
        self.samp_clk_src = None  # None (default) means using on-board sample clock
        # Start trigger
        self._export_trig = None  # None (default) means not using start trigger
        self._trig_line = None
        # External reference clock
        self._export_ref_clk = None  # None (default) means not using reference clock
        self._ref_clk_rate = None
        self._ref_clk_line = None

        self._chan_dict = {}

    def __getitem__(self, item):
        if item in self._chan_dict:
            return self._chan_dict[item]
        else:
            raise KeyError(f'There is no channel "{item}"')

    # # ToDo: implement to be able to use .keys(), .values(), and .items() to see all channels reserved
    # def __len__(self):
    #     pass
    #
    # def __iter__(self):
    #     pass

    def __repr__(self):
        return (
            f'{self.max_name}\n'
            f'\tSample clock: {self.samp_clk_info}\n'
            f'\tStart trigger: {self.trig_info}\n'
            f'\tReference clock: {self.ref_clk_info}'
        )

    @property
    def nickname(self):
        if self._nickname is not None:
            return self._nickname
        else:
            return self.max_name

    @property
    def samp_clk_info(self):
        if self.samp_clk_src is not None:
            return f'Imported {self.samp_rate:,} Hz sample clock from {self.samp_clk_src}'
        else:
            return f'Using {self.samp_rate:,} Hz onboard sample clock'

    @property
    def trig_info(self):
        if self._export_trig is True:
            return f'Exported start trigger to {self._trig_line}'
        elif self._export_trig is False:
            return f'Imported start trigger from {self._trig_line}'
        else:
            return 'Not using external start trigger'

    @property
    def ref_clk_info(self):
        if self._export_ref_clk is True:
            return f'Exported {self._ref_clk_rate*1e-6:.2f} MHz reference to {self._ref_clk_line}'
        elif self._export_ref_clk is False:
            return f'Imported {self._ref_clk_rate*1e-6:.2f} MHz reference from {self._ref_clk_line}'
        else:
            return 'Not using external reference clock'

    def cfg_samp_clk_src(self, src: str):
        self._dll.device_cfg_samp_clk_src(
            name=self.max_name,
            src=src
        )
        self.samp_clk_src = src

    def cfg_start_trig(self, line: str, export: bool = False):
        self._dll.device_cfg_trig(
            name=self.max_name,
            trig_line=line,
            export_trig=export
        )
        self._export_trig = export
        self._trig_line = line

    def cfg_ref_clk(self, line: str, rate=10e6, export: bool = False):
        self._dll.device_cfg_ref_clk(
            name=self.max_name,
            ref_clk_line=line,
            ref_clk_rate=rate,
            export_ref_clk=export,
        )
        self._export_ref_clk = export
        self._ref_clk_line = line
        self._ref_clk_rate = rate

    def clear_edit_cache(self):
        self._dll.device_clear_edit_cache(name=self.max_name)  # FixMe[Rust]: change `dev_name` to `max_name`
        self._dll.device_clear_compile_cache(name=self.max_name)  # FixMe[Rust]: change `dev_name` to `max_name`

    def reset(self):
        self._dll.reset_device(name=self.max_name)  # FixMe[Rust]: change `dev_name` to `max_name`

        # Update proxy values
        # self.samp_rate = samp_rate  # FixMe - should self.samp_rate be changed?
        # FixMe: what samp_rate is set after reset?
        #  It probably doesn't affect samp_rate,
        #  since actuall call to NIDAQmx to set samp_rate is done duting NIStreamer.stream_exp() call
        self.samp_clk_src = None
        self._export_trig = None
        self._trig_line = None
        self._export_ref_clk = None
        self._ref_clk_rate = None
        self._ref_clk_line = None


class AOCardProxy(BaseCardProxy):

    def __repr__(self):
        return 'AO card ' + super().__repr__()

    def add_chan(self, chan_idx: int, default_value: float=0., nickname: str = None):
        # Raw rust-maturin wrapper call
        self._dll.add_ao_channel(
            self.max_name, 
            channel_id=chan_idx,  # FixMe[Rust]: maybe change `channel_id` to `chan_idx`,
            default_value=default_value
        )
        # Instantiate proxy object
        chan_proxy = AOChanProxy(
            _dll=self._dll,
            _card_max_name=self.max_name,
            chan_idx=chan_idx,
            nickname=nickname
        )
        self._chan_dict[chan_proxy.chan_name] = chan_proxy
        return chan_proxy


class DOCardProxy(BaseCardProxy):

    def __repr__(self):
        return 'DO card ' + super().__repr__()

    def add_chan(self, line_idx: int, default_value: bool=False, nickname: str = None):
        self.add_chan_(line_idx // 8, line_idx % 8, default_value, nickname)

    def add_chan_(self, port_idx: int, line_idx: int, default_value: bool=False, nickname: str = None):
        # Raw rust-maturin wrapper call
        self._dll.add_do_channel(
            self.max_name, 
            port_id=port_idx,
            # FixMe[Rust]: maybe change `port_id` to `port_idx`
            #  - idx is associated with "int" - values from 0 to N-1, while "id" is more general
            line_id=line_idx,  # FixMe[Rust]: maybe change `channel_id` to `chan_idx`,
            default_value=1. if default_value else 0.
        )
        # Instantiate proxy object
        chan_proxy = DOChanProxy(
            _dll=self._dll,
            _card_max_name=self.max_name,
            port_idx=port_idx,
            line_idx=line_idx,
            nickname=nickname
        )
        self._chan_dict[chan_proxy.chan_name] = chan_proxy
        return chan_proxy
