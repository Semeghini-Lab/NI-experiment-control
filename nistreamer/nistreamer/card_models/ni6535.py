from ..card import DOCardProxy


class NI6535(DOCardProxy):
    def add_chan(self, chan_idx: int, default_value: bool = False, nickname: str = None):
        return DOCardProxy.add_chan(
            self,
            port_idx=chan_idx // 8,
            line_idx=chan_idx % 8,
            default_value=default_value,
            nickname=nickname
        )
