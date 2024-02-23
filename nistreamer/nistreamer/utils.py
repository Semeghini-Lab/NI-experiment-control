import numpy as np
from niexpctrl_backend import connect_terms as raw_connect_terms
from niexpctrl_backend import disconnect_terms as raw_disconnect_terms
from niexpctrl_backend import reset_dev as raw_reset_dev
# Import plotly
PLOTLY_INSTALLED = False
try:
    import plotly.graph_objects as go
    from plotly.subplots import make_subplots
    PLOTLY_INSTALLED = True
except ImportError:
    print(
        'Warning! Plotly package is not installed. You can still use the streamer, '
        'but plotting functionality will not be available.\n'
        'To install, run `pip install plotly` in your Python environment'
    )


def connect_terms(src: str, dest: str):
    """Statically (independently of any NI task) connect terminals

    :param src:
    :param dest:
    :return:
    """
    return raw_connect_terms(src=src, dest=dest)


def disconnect_terms(src: str, dest: str):
    return raw_disconnect_terms(src=src, dest=dest)


def share_ref_clk(dev: str, line: str):
    connect_terms(
        src=f'/{dev}/10MHzRefClock',
        dest=f'/{dev}/{line}'
    )


def reset_dev(name: str):
    return raw_reset_dev(name=name)


class RendOption:

    # Available renderers (from https://plotly.com/python/renderers/):
    #         ['plotly_mimetype', 'jupyterlab', 'nteract', 'vscode',
    #          'notebook', 'notebook_connected', 'kaggle', 'azure', 'colab',
    #          'cocalc', 'databricks', 'json', 'png', 'jpeg', 'jpg', 'svg',
    #          'pdf', 'browser', 'firefox', 'chrome', 'chromium', 'iframe',
    #          'iframe_connected', 'sphinx_gallery', 'sphinx_gallery_png']

    browser = 'browser'
    notebook = 'notebook'


def iplot(chan_list, t_start=None, t_end=None, nsamps=1000, renderer='browser', row_height=None):

    # ToDo:
    #   `src_pwr` (`slow_ao_card.ao0`) did not receive any instructions, resulting in this error
    #   PanicException: Attempting to calculate signal on not-compiled channel ao0
    #   Try checking edit cache with `is_edited`

    if not PLOTLY_INSTALLED:
        raise ImportError('Plotly package is not installed. Run `pip install plotly` to get it.')

    chan_num = len(chan_list)
    nsamps = int(nsamps)

    fig = make_subplots(
        rows=len(chan_list),
        cols=1,
        x_title='Time [s]',
        # shared_xaxes=True,  # Using this option locks X-axes,
                              # but also hides X-axis ticks for all plots except the bottom one
    )
    fig.update_xaxes(matches='x')  # Using this option locks X-axes and also leaves ticks

    if row_height is not None:
        fig.update_layout(height=1.1 * row_height * chan_num)
    else:
        # Row height is not provided - use auto-height and fit everything into the standard frame height.
        #
        # Exception - the case of many channels:
        #   - switch off auto and set fixed row height, to make frame extend downwards as much as needed
        if chan_num > 4:
            fig.update_layout(height=1.1 * 200 * chan_num)

    t_arr = None
    for idx, chan in enumerate(chan_list):

        t_start, t_end, signal_arr = chan.calc_signal(t_start=t_start, t_end=t_end, nsamps=nsamps)

        # Only compute t_arr once since it will be the same for all traces
        if t_arr is None:
            t_arr = np.linspace(t_start, t_end, nsamps)

        fig.add_trace(
            go.Scatter(
                x=t_arr,
                y=signal_arr,
                name=chan.nickname
            ),
            row=idx + 1, col=1
        )

    fig.show(renderer=renderer)
