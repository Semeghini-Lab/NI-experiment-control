from typing import Union

# Import plotly
PLOTLY_INSTALLED = False
try:
    import plotly.graph_objs
    PLOTLY_INSTALLED = True
except ImportError:
    print(
        'plotly package is not installed. You can still use the streamer, '
        'but plotting functionality will not be available.\n'
        'To install, run `pip install plotly` in your Python environment'
    )


def _iplot(
        chan_obj: Union[NIStreamer.AOCard.OutChnl, NIStreamer.DOCard.OutChnl],
        t_stat=None,
        t_end=None,
        nsamps=1000
):
    if not PLOTLY_INSTALLED:
        raise RuntimeError('Plotly package is not installed. Run `pip install plotly` to get it.')

    signal_arr = chan_obj.calc_signal(t_start=t_stat, t_end=t_end, nsamps=nsamps)
    # trace = plotly.graph_objs.Scatter()



def iplot(obj, t_stat=None, t_stop=None, renderer='HTML'):
    # FixMe[Rust]: currently, `calc_signal()` is implemented for device-level.
    #  Re-implement for channel-level

    if not isinstance(obj, (NIStreamer.AOCard, NIStreamer.DOCard)):
        raise NotImplementedError('Temporary: only implemented for card-level')

    pass