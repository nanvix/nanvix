import sys
import argparse
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator

file_name1024 = "./csvdata/parsedcsv/parsed_nanvix-boot_time-1024.csv"
file_name512 = "./csvdata/parsedcsv/parsed_nanvix-boot_time-512.csv"
file_name256 = "./csvdata/parsedcsv/parsed_nanvix-boot_time-256.csv"
file_name128 = "./csvdata/parsedcsv/parsed_nanvix-boot_time-128.csv"
file_name64 = "./csvdata/parsedcsv/parsed_nanvix-boot_time-64.csv"

funcname="vm_run"

def parse_file(file_name):
    df = pd.read_csv(file_name, sep=',')
    df.columns = ["Memory Size", "TimeMicroSec"]
    return df

def plot_cumulative_sum(vet):
    for df,label in vet:
        mean = df['TimeMicroSec'].mean() / 1000
        legend = label + " - " + str(f"{mean:.3f}") + "ms"

        df = df / 1000

        # sort the data:
        df_sorted = np.sort(df['TimeMicroSec'])

        # calculate the proportional values of samples
        p = 1. * np.arange(len(df_sorted)) / (len(df_sorted) - 1)

        # plot the sorted data:
        plt.plot(df_sorted, p, label=legend)
        plt.legend()
        plt.grid()

        plt.xlabel(f"${funcname} Time$ (ms)")
        plt.ylabel('$Cumulative Probability$')

    plt.savefig("csvdata/plots/" + "plot.png")

def main() -> None:
    vet = []
    vet.append((parse_file(file_name1024), "1024M"))
    vet.append((parse_file(file_name512) , "512M" ))
    vet.append((parse_file(file_name256) , "256M" ))
    vet.append((parse_file(file_name128) , "128M" ))
    vet.append((parse_file(file_name64) , "64M" ))

    plot_cumulative_sum(vet)

if __name__ == "__main__":
    main()
