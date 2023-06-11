#!/usr/bin/env python

import os
import sys
import matplotlib.pyplot as plt
import pandas
import numpy
import math

colors = [(0.8, 0.3, 0.3), (1, 0.6, 0.3), (0.3, 0.8, 0.3), (0.3, 0.3, 0.8)]

plugin_labels = ['Default', 'Edge with No Compression', 'Edge with Level 2 Compression', 'Edge with Level 9 Compression']
plugins = ['default', 'server_none', 'server_2', 'server_9'] # ['default', 'server_none', 'server_2', 'server_9']
num_objects = [500, 1000, 2000, 4000, 8000]
shapes = ['ball', 'cuboid', 'capsule', 'complex']

# call signature ./plot.py <input_dir> <output_dir>
def main():
    if len(sys.argv) < 3:
        print('Invalid number of arguments')
        return

    input_dir = sys.argv[1]
    output_dir = sys.argv[2]

    plugin_fps = []

    for plugin in plugins:
        fps_lines = []
        for shape in shapes:
            fps_line = []
            for num_object in num_objects:
                config = f'{plugin}_{num_object}_{shape}'
                try:
                    dataframe = pandas.read_csv(f'{input_dir}/{config}')
                except FileNotFoundError:
                    print(f'file {input_dir}/{config} could not be found')
                    fps_line.append(0)
                    continue
                
                fps_line.append(dataframe['fps'][1:].mean())

            fps_lines.append(fps_line)
        plugin_fps.append(fps_lines)

    figures = []
    
    """
    Plot shapes per solution
    """
    fig, axs = plt.subplots(2, math.ceil(len(plugin_fps) / 2))
    figures.append(('shapes-per-solution', fig))

    for (idx, fps_lines) in enumerate(plugin_fps):
        ax = axs[int(idx / 2), idx % 2]
        ax.set_title(plugin_labels[idx])
        ax.set_ylim(bottom=0, top=800)
        ax.set_xlim(left=0,right=8500)
        ax.set_xlabel('Number of Objects')
        ax.set_ylabel('Frame per Second (FPS)')

        for (shape_idx, fps_line) in enumerate(fps_lines):
            ax.plot(num_objects, fps_line, linewidth=2.0, label=shapes[shape_idx], color=colors[shape_idx])

        ax.legend()

    """
    Plot solutions per shape
    """
    fig, axs = plt.subplots(2, math.ceil(len(shapes) / 2))
    figures.append(('solutions-per-shape', fig))

    xvalues = ['Low # of Objects (1000)', 'Medium # of Objects (4000)', 'High # of Objects (8000)']

    for shape_idx in range(len(shapes)):
        ax = axs[int(shape_idx / 2), shape_idx % 2]
        ax.set_title(f'{shapes[shape_idx]} behaviour across solutions')
        ax.set_ylim(bottom=0, top=800)
        ax.set_xlabel('Number of Objects')
        ax.set_ylabel('Frame per Second (FPS)')
        ax.set_xticks(range(0, 3), labels=xvalues)

        for (idx, fps_lines) in enumerate(plugin_fps):
            yvalues = [fps_lines[shape_idx][1], fps_lines[shape_idx][3], fps_lines[shape_idx][4]]
            positions = list(map(lambda x: x + (idx - 2) * 0.15, range(0, 3)))
            ax.bar(positions, yvalues, width=0.1, color=colors[idx], label=plugin_labels[idx])

        ax.legend()

    """
    Plot the network versus physics time for ball
    """
    fig, axs = plt.subplots(2, math.ceil(len(plugins) / 2))
    figures.append(('network-physics-time', fig))
    
    for (idx, plugin) in enumerate(plugins):
        ax = axs[int(idx / 2), idx % 2]
        shape = 'ball'
        network_line = []
        physics_line = []

        ax.set_title(plugin_labels[idx])
        ax.set_xlabel('Number of Objects')
        ax.set_ylabel('Time(ms)')

        for num_object in num_objects:
            config = f'{plugin}_{num_object}_{shape}'
            try:
                dataframe = pandas.read_csv(f'{input_dir}/{config}')
            except FileNotFoundError:
                print(f'file {input_dir}/{config} could not be found')
                network_line.append(0)
                physics_line.append(0)
                continue

            physics_mean = dataframe['physics_time'][1:].mean()
            network_line.append((dataframe['network_time'][1:].mean() - physics_mean) / 1000)
            physics_line.append(physics_mean / 1000)

        ax.plot(num_objects, network_line, linewidth=2.0, label='Network Time + Compression', color=colors[0])
        ax.plot(num_objects, physics_line, linewidth=2.0, label='Physics Time', color=colors[3])
        ax.legend()

    plt.plot()

    plt.show()

    for (name, fig) in figures:
        fig.savefig(name, dpi=300)

    a = 2
                
if __name__ == '__main__':
    main()
