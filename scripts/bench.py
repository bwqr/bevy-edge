#!/usr/bin/env python
import os
import time
import sys
import subprocess
import multiprocessing

iterations = range(1) # 2
plugins = ['server_none'] # ['server_none', 'server_1', 'server_3', 'server_9'] # ['default', 'server_none', 'server_2', 'server_9']
num_objects = [500] # [500, 1000, 2000, 4000, 8000]
shapes = ['ball'] # ['ball', 'capsule', 'cuboid', 'complex']

# call signature ./bench.py <configs-dir> <output-dir> program
def main():
    if len(sys.argv) < 5:
        print('Invalid number of arguments')
        return

    configs_path = sys.argv[1]
    output_path = sys.argv[2]
    bench_output_path = sys.argv[3]
    program = sys.argv[4]
    physics_program = program.find('physics') > -1

    timestamp = int(time.time() * 1000)
    cpu_count = multiprocessing.cpu_count()

    configs = []
    for plugin in plugins:
        for num_object in num_objects:
            for shape in shapes:
                for i in iterations:
                    configs.append((i, f'{plugin}_{num_object}_{shape}'))

    run_config_num = 0
    for (iteration, config) in configs:
        if not os.path.isfile(f'{configs_path}/{config}'):
            continue

        run_config_num += 1

        print(f'Running the config {config} iter {iteration}')

        if physics_program:
            bevy_output = open(f'{output_path}/{run_config_num}', mode='w')
            bench_output = open(f'{bench_output_path}/{run_config_num}', mode='w')
        else:
            bevy_output = open(f'{output_path}/{config}_{iteration}', mode='w')
            bench_output = open(f'{bench_output_path}/{config}_{iteration}', mode='w')

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
        bench_output.close()


if __name__ == '__main__':
    main()
