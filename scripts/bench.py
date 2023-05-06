#!/usr/bin/env python
import os
import time
import sys
import subprocess
import multiprocessing

plugins = ['server_2'] # ['default', 'server_none', 'server_2', 'server_9']
num_objects = [1000, 2000, 4000, 8000]
shapes = ['ball', 'cuboid', 'capsule']

# call signature ./bench.py <configs-dir> <output-dir> program
def main():
    if len(sys.argv) < 4:
        print('Invalid number of arguments')
        return

    configs_path = sys.argv[1]
    output_path = sys.argv[2]
    program = sys.argv[3]

    timestamp = int(time.time() * 1000)
    cpu_count = multiprocessing.cpu_count()

    configs = []
    for plugin in plugins:
        for num_object in num_objects:
            for shape in shapes:
                configs.append(f'{plugin}_{num_object}_{shape}')

    for config in configs:
        if not os.path.isfile(f'{configs_path}/{config}'):
            continue

        bevy_output = open(f'{output_path}/{config}', mode='w')
        bench_output = sys.stdout

        handle = subprocess.Popen([program, f'{configs_path}/{config}'], stdout=bevy_output)
        top_handle = subprocess.Popen(['top', '-p', f'{handle.pid}', '-b', '-d' '0.05'], stdout=subprocess.PIPE)

        start_time = int(time.time() * 1000)

        while handle.poll() is None:
            for _ in range(7):
                top_handle.stdout.readline()
            stats = top_handle.stdout.readline().split()
            top_handle.stdout.readline()
            mem = stats[5].decode('utf-8')
            cpu = round(float(stats[8]) / cpu_count, 2)
            diff_time = int(time.time() * 1000) - start_time
            bench_output.write(f'{diff_time},{mem},{cpu}\n')

        top_handle.terminate()

        bevy_output.close()
        #bench_output.close()

if __name__ == '__main__':
    main()
