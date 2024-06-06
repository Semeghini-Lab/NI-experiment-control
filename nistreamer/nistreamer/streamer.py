from niexpctrl_backend import Experiment as RawStreamer
from .card import AOCardProxy, DOCardProxy
from typing import Optional, Literal, Union, Tuple


class NIStreamer:

    def __init__(self):
        self._streamer = RawStreamer()
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
            raw_streamer_method = RawStreamer.add_ao_device
            proxy_class = AOCardProxy
            target_dict = self._ao_card_dict
        elif card_type == 'DO':
            raw_streamer_method = RawStreamer.add_do_device
            proxy_class = DOCardProxy
            target_dict = self._do_card_dict
        else:
            raise ValueError(f'Invalid card type "{card_type}". Valid type strings are "AO" and "DO"')

        # Raw (maturin wrapped) DLL call
        raw_streamer_method(
            self._streamer,
            max_name,  # FixMe[Rust]: change `physical_name` to `max_name`
            samp_rate=samp_rate
        )
        # Proxy object
        proxy = proxy_class(
            _streamer=self._streamer,
            max_name=max_name,
            nickname=nickname
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

    @property
    def starts_last(self) -> Union[str, None]:
        """Specifies which card starts last. Typically, this is needed when start trigger or shared sample clock are used
        for hardware synchronisation.

        Format:
            * `dev_name: str` - this card will wait for all other ones to start first;
            * `None` - each threads will start its task whenever it is ready without waiting for anyone.

        Specifically, it determines which thread waits to call `ni_task.start()` until all other threads have called
        their `ni_task.start()` first.

        If a given card is awaiting a start trigger / uses external sample clock,
        it should start before the card which produces this signal (the 'primary card'),
        otherwise the trigger pulse / first few clock pulses can be missed.

        Streamer provides option to specify a single primary card and handles the necessary thread sync
        to make sure the designated card calls `ni_task.start()` last.
        """
        return self._streamer.get_starts_last()

    @starts_last.setter
    def starts_last(self, name: Union[str, None]):
        self._streamer.set_starts_last(name=name)

    @property
    def ref_clk_provider(self) -> Union[Tuple[str, str], None]:
        """Specifies which card exports its 10MHz reference signal for use by all other cards.

        Format:
            * `(dev_name: str, term_name: str)` - card `dev_name` exports 10MHz ref to terminal `term_name`
            * `None` - no card exports

        Technical details:
            (1) NIStreamer uses 'run-based' static reference clock export: signal is exported during `cfg_run()`
            and un-exported during `close_run()` calls. This export is not dependent on any NI tasks.

            As a result, the provider can be any card supporting 10MHz ref export.
            It does not have to get any instructions or even be registered in the `NIStreamer`.

            (2) Users can manually do static export of 10MHz ref from any card by calling `utils.share_10mhz_ref()`.
            However, such export will not be automatically undone and the user has to manually call
            either `utils.unshare_10mhz_ref()` or `utils.reset_dev()`.

            This is dangerous since forgetting to un-export can easily lead to foot guns. Only do this if you
            need to go beyond standard configuration. For most cases just specify `ref_clk_provider` since it does
            precisely that but also does everything possible to automatically undo the export whenever the run stops.
        """
        return self._streamer.get_ref_clk_provider()

    @ref_clk_provider.setter
    def ref_clk_provider(self, dev_and_term: Union[Tuple[str, str], None]):
        self._streamer.set_ref_clk_provider(provider=dev_and_term)

    def compile(self, stop_time: Optional[float] = None) -> float:
        return self._streamer.compile(stop_time=stop_time)

    def run(self, nreps: Optional[int] = 1,  bufsize_ms: Optional[float] = 150) -> None:
        try:
            self._streamer.cfg_run(bufsize_ms=bufsize_ms)
            for i in range(nreps):
                self._streamer.stream_run(calc_next=(i < nreps - 1))
        except KeyboardInterrupt:
            pass
        finally:
            self._streamer.close_run()

    def add_reset_instr(self, reset_time: Optional[float] = None):
        self._streamer.add_reset_instr(reset_time=reset_time)

    def clear_edit_cache(self):
        self._streamer.clear_edit_cache()
        self._streamer.clear_compile_cache()

    def reset_all(self):
        for card_group in [self._ao_card_dict.values(), self._do_card_dict.values()]:
            for card in card_group:
                card.reset()
