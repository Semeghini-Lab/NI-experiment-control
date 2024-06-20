from nistreamer.card import BaseCardProxy, DOCardProxy
from nistreamer.channel import DOChanProxy


class NI6535(DOCardProxy):
    def __repr__(self):
        return 'DO card NI6535 ' + BaseCardProxy.__repr__(self)

    def add_chan(self, chan_idx: int, default_value: bool = False, nickname: str = None, proxy_class=DOChanProxy):
        return super().add_chan(
            port_idx=chan_idx // 8,
            line_idx=chan_idx % 8,
            default_value=default_value,
            nickname=nickname,
            proxy_class=proxy_class
        )
