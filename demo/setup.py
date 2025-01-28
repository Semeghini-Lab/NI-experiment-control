import numpy as np
from niexpctrl_backend import Experiment
from plotly.subplots import make_subplots
from plotly import graph_objects

# NI-experiment-control is installed localy at C:\Users\exp-control\Documents\NI-experiment-control

# region ---------- Initialize NI-Experiment-Control ----------

'''
Initialize NI experiment control system. To edit the NI-cards being initialized or
to edit the channels on each card being initialized, navigate to the relevant section
of this region and perform edits.
'''
exp = Experiment()

TRIG_LINE = 'PXI_Trig0'
REF_CLK_LINE = 'PXI_Trig7'

# ----- Add first analog card (the "primary card") -----
# Note a primary card is required to trigger and sync all other cards
primary_card = 'PXI1Slot2'
exp.add_ao_device(name=primary_card, samp_rate=1e6)

# Configure the primary card to provide both the start trigger and 10 MHz reference clock for all other cards
exp.dev_set_start_trig_out(name=primary_card, term=TRIG_LINE)
exp.set_ref_clk_provider(provider=(primary_card, REF_CLK_LINE))
# This card should always start last so that other cards have been initialized when this card sends the start trigger
exp.set_starts_last(primary_card)

print(f'Primary NI control card is {primary_card}')

# ----- Add remaining analog cards -----
'''
For NI PXIe-6739 the max sampling rates are:
- 1-8 channels (1 per bank of 4 consecutive channels): 1e6 Hz max
- otherwise: 0.4e6 Hz max

Note: remember to configure the card to accept starting trigger and clock from the `primary_card`
'''

exp.add_ao_device(
        name='PXI1Slot3', 
        samp_rate=1e6)
exp.dev_set_start_trig_in(name='PXI1Slot3', term=TRIG_LINE)
exp.dev_set_ref_clk_in(name='PXI1Slot3', term=REF_CLK_LINE)


# ----- Add remaining digital cards -----
'''
Note: remember to configure the card to accept starting trigger and clock from the `primary_card`
'''
exp.add_do_device(
        name='PXI1Slot7', 
        samp_rate=10e6)
exp.dev_set_start_trig_in(name='PXI1Slot7', term=TRIG_LINE)
exp.dev_set_samp_clk_in(name='PXI1Slot7', term=REF_CLK_LINE)

print(f'All NI cards initialized')

# region ---------- Initialize all channels ----------

def add_ao_channel(card_name, channel):
    '''
    card_name: name of the card in NI MAX
    channel: for NI-6739 the channels are numbered AO 0-63
    '''
    exp.add_ao_channel(name=card_name, channel_id=channel, default_value=0.0)
    return (card_name, f'ao{channel}')

def add_do_channel(card_name, port, line):
    '''
    card_name: name of the card in NI MAX
    channel: for NI-6535 the channels are numbered 0-3.0.7 in port.line format
    '''
    exp.add_do_channel(name=card_name, port_id=port, line_id=line, default_value=0)
    return (card_name, f'port{port}/line{line}')

# TODO: define all channels to be used in the experiment sequence
MOT_AOM_1 = add_ao_channel('PXI1Slot2', 0)
MOT_AOM_2 = add_ao_channel('PXI1Slot3', 0)
MOT_AOM_3 = add_ao_channel('PXI1Slot3', 1)
coil_trigger = add_do_channel('PXI1Slot7', 0, 0)

print(f'All NI channels added')

# region ---------- NI helpers ----------

def run_sequence(nreps=1, bufsize_ms=150):
    '''
    Paramters:
    nreps: number of repetitions
    bufsize_ms: size of streaming buffer in ms
    '''
    exp.compile(stop_time=None)
    
    try:
        exp.cfg_run(bufsize_ms=150)
        for i in range(nreps):
            exp.stream_run(calc_next=(i < nreps - 1))
    finally:
        exp.close_run()

def plot_sequence(channels, t_start=0, t_end=None, samp_rate=1e6, renderer='vscode'):
    '''
    Paramters:
    Arr: of channels to be plotted
    t_start: start time in seconds
    t_end: if None, t_end is the end of the sequence
    samp_rate: resolution of the plot
    renderer: available renderers (from https://plotly.com/python/renderers/):
              ['plotly_mimetype', 'jupyterlab', 'nteract', 'vscode',
               'notebook', 'notebook_connected', 'kaggle', 'azure', 'colab',
               'cocalc', 'databricks', 'json', 'png', 'jpeg', 'jpg', 'svg',
               'pdf', 'browser', 'firefox', 'chrome', 'chromium', 'iframe',
               'iframe_connected', 'sphinx_gallery', 'sphinx_gallery_png']
    '''
    if not t_end:
        t_end = exp.last_instr_end_time()
    nsamps = int((t_end-t_start)*samp_rate)

    fig = make_subplots(rows=len(channels), cols=1, x_title='Time [ms]')
    fig.update_xaxes(matches='x')  # Using this option locks X-axes and also leaves ticks
    fig.update_layout(height=1.1 * 200 * len(channels))

    t_arr = np.linspace(t_start, t_end, nsamps)*1e3

    # this is super janky
    def get_var_name(var):
        for name, value in globals().items():
            if value is var:
                return name

    row = 1
    for chan in channels:
        fig.add_trace(
            graph_objects.Scatter(
                x=t_arr,
                y=exp.channel_calc_signal_nsamps(*chan, start_time=t_start, end_time=t_end, num_samps=nsamps),
                name=get_var_name(chan)
            ),row=row, col=1)
        row += 1

    fig.show(renderer=renderer)