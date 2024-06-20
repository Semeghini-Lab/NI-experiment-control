from niexpctrl_backend import Experiment as RawStreamer
from .channel import AOChanProxy, DOChanProxy
from .utils import reset_dev
from typing import Union


class BaseCardProxy:

    def __init__(
            self,
            _streamer: RawStreamer,
            max_name: str,
            nickname=None
    ):
        self._streamer = _streamer
        self.max_name = max_name
        self._nickname = nickname

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
            f'\n'
            f'Channels: {list(self._chan_dict.keys())}\n'
            f'\n'
            f'Hardware settings:\n'
            f'\tSample rate: {self.samp_rate:,} Sa/s\n'
            f'\n'
            f'\tStart trigger: \n'
            f'\t\t in: {self.start_trig_in}\n'
            f'\t\tout: {self.start_trig_out}\n'
            f'\tSample clock:\n'
            f'\t\t in: {self.samp_clk_in}\n'
            f'\t\tout: {self.samp_clk_out}\n'
            f'\t10 MHz reference clock: \n'
            f'\t\t in: {self.ref_clk_in}\n'
            f'\t\tout: see NIStreamer.ref_clk_provider setting\n'
            f'\n'
            f'\tMin buffer write timeout: {self.min_bufwrite_timeout} sec'
        )

    @property
    def nickname(self):
        if self._nickname is not None:
            return self._nickname
        else:
            return self.max_name

    # region Hardware settings
    @property
    def samp_rate(self) -> float:
        return self._streamer.dev_get_samp_rate(name=self.max_name)

    # - Sync settings:
    @property
    def start_trig_in(self) -> Union[str, None]:
        return self._streamer.dev_get_start_trig_in(name=self.max_name)
    @start_trig_in.setter
    def start_trig_in(self, term: Union[str, None]):
        self._streamer.dev_set_start_trig_in(name=self.max_name, term=term)

    @property
    def start_trig_out(self) -> Union[str, None]:
        return self._streamer.dev_get_start_trig_out(name=self.max_name)
    @start_trig_out.setter
    def start_trig_out(self, term: Union[str, None]):
        self._streamer.dev_set_start_trig_out(name=self.max_name, term=term)

    @property
    def samp_clk_in(self) -> Union[str, None]:
        return self._streamer.dev_get_samp_clk_in(name=self.max_name)
    @samp_clk_in.setter
    def samp_clk_in(self, term: Union[str, None]):
        self._streamer.dev_set_samp_clk_in(name=self.max_name, term=term)

    @property
    def samp_clk_out(self) -> Union[str, None]:
        return self._streamer.dev_get_samp_clk_out(name=self.max_name)
    @samp_clk_out.setter
    def samp_clk_out(self, term: Union[str, None]):
        self._streamer.dev_set_samp_clk_out(name=self.max_name, term=term)

    @property
    def ref_clk_in(self) -> Union[str, None]:
        return self._streamer.dev_get_ref_clk_in(name=self.max_name)
    @ref_clk_in.setter
    def ref_clk_in(self, term: Union[str, None]):
        self._streamer.dev_set_ref_clk_in(name=self.max_name, term=term)

    # - Buffer write settings:
    @property
    def min_bufwrite_timeout(self) -> Union[float, None]:
        return self._streamer.dev_get_min_bufwrite_timeout(name=self.max_name)
    @min_bufwrite_timeout.setter
    def min_bufwrite_timeout(self, min_timeout: Union[str, None]):
        self._streamer.dev_set_min_bufwrite_timeout(name=self.max_name, min_timeout=min_timeout)
    # endregion

    def clear_edit_cache(self):
        self._streamer.device_clear_edit_cache(name=self.max_name)
        self._streamer.device_clear_compile_cache(name=self.max_name)

    def reset(self):
        reset_dev(name=self.max_name)

    def last_instr_end_time(self):
        return self._streamer.device_last_instr_end_time(
            self.max_name
        )


class AOCardProxy(BaseCardProxy):

    def __repr__(self):
        return 'AO card ' + super().__repr__()

    def add_chan(self, chan_idx: int, default_value: float = 0., nickname: str = None):
        # Raw rust-maturin wrapper call
        self._streamer.add_ao_channel(
            self.max_name, 
            channel_id=chan_idx,  # FixMe[Rust]: maybe change `channel_id` to `chan_idx`,
            default_value=default_value
        )
        # Instantiate proxy object
        chan_proxy = AOChanProxy(
            _streamer=self._streamer,
            _card_max_name=self.max_name,
            chan_idx=chan_idx,
            nickname=nickname
        )
        self._chan_dict[chan_proxy.chan_name] = chan_proxy
        return chan_proxy


class DOCardProxy(BaseCardProxy):

    def __repr__(self):
        return 'DO card ' + super().__repr__()

    def add_chan(self, chan_idx: int, default_value: bool = False, nickname: str = None):
        return self.add_chan_(chan_idx // 8, chan_idx % 8, default_value, nickname)

    def add_chan_(self, port_idx: int, line_idx: int, default_value: bool = False, nickname: str = None):
        # Raw rust-maturin wrapper call
        self._streamer.add_do_channel(
            self.max_name, 
            port_id=port_idx,
            # FixMe[Rust]: maybe change `port_id` to `port_idx`
            #  - idx is associated with "int" - values from 0 to N-1, while "id" is more general
            line_id=line_idx,  # FixMe[Rust]: maybe change `channel_id` to `chan_idx`,
            default_value=1. if default_value else 0.
        )
        # Instantiate proxy object
        chan_proxy = DOChanProxy(
            _streamer=self._streamer,
            _card_max_name=self.max_name,
            port_idx=port_idx,
            line_idx=line_idx,
            nickname=nickname
        )
        self._chan_dict[chan_proxy.chan_name] = chan_proxy
        return chan_proxy
