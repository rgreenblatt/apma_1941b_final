import argparse

import pandas as pd
import matplotlib.pyplot as plt


def main():
    parser = argparse.ArgumentParser(
        description='plot a contribution distribution')
    parser.add_argument('csv_path')
    parser.add_argument('output_path')
    parser.add_argument('--title-name')

    args = parser.parse_args()

    df = pd.read_csv(args.csv_path, sep=",")
    contribution_nums = df['num']
    counts = df['count']

    plt.plot(contribution_nums, counts / counts.sum())
    title_name = args.title_name
    if title_name is None:
        title_name = ""
    else:
        title_name = " " + title_name
    plt.title("contribution distribution" + title_name)
    plt.xscale('log')
    plt.yscale('log')
    plt.savefig(args.output_path)


if __name__ == "__main__":
    main()
