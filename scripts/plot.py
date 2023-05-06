#!/usr/bin/env python

import os
import sys
import matplotlib.pyplot as plt
import pandas
import numpy
import math

plugins = ['default', 'server_none', 'server_2', 'server_9'] # ['default', 'server_none', 'server_2', 'server_9']
num_objects = [500, 1000, 2000, 4000, 8000]
shapes = ['ball', 'cuboid', 'capsule']

# call signature ./plot.py <input_dir> <output_dir>
def main():
    if len(sys.argv) < 3:
        print('Invalid number of arguments')
        return

    input_dir = sys.argv[1]
    output_dir = sys.argv[2]

    plugin_lines = []

    for plugin in plugins:
        lines = []
        for shape in shapes:
            line = []
            for num_object in num_objects:
                config = f'{plugin}_{num_object}_{shape}'
                try:
                    dataframe = pandas.read_csv(f'{input_dir}/{config}')
                except FileNotFoundError:
                    print(f'file {input_dir}/{config} could not be found')
                    line.append(0)
                    continue
                
                line.append(dataframe['fps'][1:].mean())

            lines.append(line)
        plugin_lines.append(lines)

    
    fig, axs = plt.subplots(2, math.ceil(len(plugin_lines) / 2))
    for (idx, lines) in enumerate(plugin_lines):
        ax = axs[int(idx / 2), idx % 2]
        ax.set_title(plugins[idx])

        for (shape_idx, line) in enumerate(lines):
            drawn_line, = ax.plot(num_objects, line, linewidth=2.0)
            drawn_line.set_label(shapes[shape_idx])

        ax.set_xlabel('Number of Objects')
        ax.set_ylabel('Frame per Second (FPS)')
        ax.legend()


    plt.show()
    a = 2
                
if __name__ == '__main__':
    main()
