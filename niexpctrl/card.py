from niexpctrl_backend import Experiment as RawDLL
from channel import AOChanProxy, DOChanProxy
from typing import Optional, Union, Literal


class BaseCardProxy:

    def __init__(
            self,
            _dll: RawDLL,
            max_name: str,
            nickname=None,
            config_info: str = None
    ):
        self._dll = _dll
        self.max_name = max_name
        self._nickname = nickname
        self._config_info = config_info
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
            f'config_info: {self._config_info}'
        )

    @property
    def nickname(self):
        if self._nickname is not None:
            return self._nickname
        else:
            return self.max_name

    def clear_edit_cache(self):
        self._dll.device_clear_edit_cache(dev_name=self.max_name)  # FixMe[Rust]: change `dev_name` to `max_name`
        self._dll.device_clear_compile_cache(dev_name=self.max_name)  # FixMe[Rust]: change `dev_name` to `max_name`

    def reset(self):
        self._dll.reset_device(dev_name=self.max_name)  # FixMe[Rust]: change `dev_name` to `max_name`


class AOCardProxy(BaseCardProxy):

    def __repr__(self):
        return 'AO card ' + super().__repr__()

    def add_chnl(self, chan_idx: int, nickname: str = None):
        # Raw rust-maturin wrapper call
        self._dll.add_ao_channel(
            dev_name=self.max_name,  # FixMe[Rust]: change `dev_name` to `max_name`
            channel_id=chan_idx,  # FixMe[Rust]: maybe change `channel_id` to `chan_idx`
        )
        # Instantiate proxy object
        chan_obj = AOChanProxy(
            _dll=self._dll,
            _card_max_name=self.max_name,
            chan_idx=chan_idx,
            nickname=nickname
        )
        self._chan_dict[chan_idx] = chan_obj
        return chan_obj


class DOCardProxy(BaseCardProxy):

    def __repr__(self):
        return 'DO card ' + super().__repr__()

    def add_chnl(self, port_idx: int, line_idx: int, nickname: str = None):
        # Raw rust-maturin wrapper call
        self._dll.add_do_channel(
            dev_name=self.max_name,  # FixMe[Rust]: change `dev_name` to `max_name`
            port_id=port_idx,
            # FixMe[Rust]: maybe change `port_id` to `port_idx`
            #  - idx is associated with "int" - values from 0 to N-1, while "id" is more general
            line_id=line_idx  # FixMe[Rust]: maybe change `channel_id` to `chan_idx`
        )
        # Instantiate proxy object
        chnl_obj = DOChanProxy(
            _dll=self._dll,
            _card_max_name=self.max_name,
            port_idx=port_idx,
            line_idx=line_idx,
            nickname=nickname
        )
        self._chan_dict[chnl_obj.chan_name] = chnl_obj
        return chnl_obj
