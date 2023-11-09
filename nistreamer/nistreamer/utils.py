import numpy as np
from typing import Union

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


# Utilities to save a notebook
import os
import shutil
from IPython.display import display, Javascript
import ipykernel
import json
import requests
from datetime import datetime
from notebook import notebookapp

# To use the function, you call it with the directory and name you want.
# For example, to save to the directory '/target/directory' with the name 'my_copy':
# save_notebook('/target/directory', 'my_copy')
def save_notebook_(target_dir, custom_name):
    # First, we save the current state of the notebook to ensure it's up-to-date.
    display(Javascript('IPython.notebook.save_checkpoint();'))
    
    # We need to find the path to the current notebook, which requires access to the notebook server.
    # This code will get the current notebook kernel's connection file
    connection_file = os.path.basename(ipykernel.get_connection_file())
    # The kernel id is part of the connection file name
    kernel_id = connection_file.split('-', 1)[1].split('.')[0]

    # Now we can iterate through the running servers and find the one that matches the kernel id
    for srv in notebookapp.list_running_servers():
        try:
            # The notebook server provides an API to list the current notebooks
            response = requests.get(f'{srv["url"]}api/sessions', headers={'Authorization': f'token {srv["token"]}'}, verify=False)
            response.raise_for_status()
            # We parse the response as JSON
            sessions = json.loads(response.text)
            # And look for the notebook with the matching kernel id
            for sess in sessions:
                if sess['kernel']['id'] == kernel_id:
                    # Once we find it, we can get the notebook path
                    notebook_path = sess['notebook']['path']
                    # We construct the full path to the notebook file
                    notebook_dir = os.path.dirname(notebook_path)
                    full_path = os.path.join(srv['notebook_dir'], notebook_path)
                    # And the target path where we want to save the copy
                    target_path = os.path.join(target_dir, f'{custom_name}.ipynb')
                    # Now we copy the notebook to the target directory with the new name
                    shutil.copy2(full_path, target_path)
                    return
        except Exception as e:
            # If there's any error, we print it
            print(f'Error: {e}')

def checkpoint_notebook(save_dir, name):
    # Get the current datetime
    now = datetime.now()
    # Format the directory structure
    date_dir = now.strftime("%Y/%m/%d")
    # Create the target directory if it doesn't exist
    target_dir = os.path.join(save_dir, date_dir)
    os.makedirs(target_dir, exist_ok=True)
    # Format the filename with the current time
    time_str = now.strftime("%H-%M-%S")
    filename = f"{name}-{time_str}.ipynb"
    # Call save_notebook to save the file in the structured directory
    save_notebook_(target_dir, filename)
    # Echo back the absolute path of the file saved
    saved_file_path = os.path.abspath(os.path.join(target_dir, filename))
    print(f"File saved at: {saved_file_path}")
    return saved_file_path