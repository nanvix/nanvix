#!/bin/bash

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

# Nanvix Directory
NANVIX_PATH=${1:-$(pwd)}

# Binaries Directory.
BINARIES_PATH=${2:-$(pwd)/bin}

# Data Directory.
DATA_PATH=${3:-$NANVIX_PATH/csvdata}

# Binaries name.
KERNEL=kernel.elf
MICROVM=microvm.elf
APPLICATION=boottime.elf

# CPU Masks
NANVIX_CPU_MASK=0,1

# Run each test
MANY_TIME=500

# Nanvix memory size
MEMORY_SIZE=(64 128 256 512 1024)

function run_nanvix {
    sudo -E nice -n -20 taskset -a -c ${NANVIX_CPU_MASK} ${BINARIES_PATH}/${MICROVM} \
           -kernel ${BINARIES_PATH}/${KERNEL}                                       \
           -initrd ${BINARIES_PATH}/${APPLICATION}                                     \
           -memory ${MEMORY_SIZE}M                                                  \
            2>> ${DATA_PATH}/originalcsv/nanvix-boot_time-${MEMORY_SIZE}.csv
}

function print_progress {
    local status=$(( ${IT} * 100 / ${MANY_TIME} ))

    echo -e ${MEMORY_SIZE}M
    echo -e Running: ${status}% '\r'
}

function compile_nanvix {
    local mem_size_byte=$((MEMORY_SIZE*1048576)) 

    # Configure kernel memory size in bytes.
    sed -i -e "s/^memory_size = [0-9]*$/memory_size = ${mem_size_byte}/" ${NANVIX_PATH}/build/kernel_config.toml

    make -C ${NANVIX_PATH} clean

    # Compile kernel, microvm and applications.
    make -C ${NANVIX_PATH} MACHINE=microvm TARGET=x86 LOG_LEVEL=error RELEASE=yes PROFILER=yes all
}

function parse_boottime {
    for memory in "${MEMORY_SIZE[@]}";
        do
            file=${DATA_PATH}/originalcsv/nanvix-boot_time-${memory}.csv
            paste -d ',' \
                <(cat $file | grep '^\+,vm_run' | cut -d',' -f 5) \
                <(cat $file | grep "^\+,vmm_creation" | cut -d',' -f 5) \
                | awk -F',' '{print $1 + $2}' \
                | sed "s/^/${memory},/g" \
                >> ${DATA_PATH}/parsedcsv/parsed_nanvix-boot_time-${memory}.csv
        done
}

# Clean environment.
function clean {
    make -C ${NANVIX_PATH} clean
    rm -rf ${DATA_PATH}
}

# Start Script.
clean

mkdir -p ${DATA_PATH}
mkdir -p ${DATA_PATH}/originalcsv
mkdir -p ${DATA_PATH}/parsedcsv
mkdir -p ${DATA_PATH}/plots

for MEMORY_SIZE in 64 128 256 512 1024;
do
    compile_nanvix $MEMORY_SIZE
    for IT in $( eval echo {1..$MANY_TIME} );
    do
        run_nanvix
        print_progress $IT $MEMORY_SIZE
    done
done

parse_boottime

python3 ${NANVIX_PATH}/scripts/plot-boot_time.py
