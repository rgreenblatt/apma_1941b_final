import os

import pandas as pd
import matplotlib.pyplot as plt

FIG_DIR = 'figs/'
OUTPUT_DATA_DIR = 'output_data/'


def main():
    os.makedirs(FIG_DIR, exist_ok=True)

    for name, title in [("user_degrees", "user degrees"),
                        ("repo_degrees", "repo degrees"),
                        ("user_total_contributions",
                         "user total contributions"),
                        ("repo_total_events", "repo total events")]:

        df = pd.read_csv("{}/{}.csv".format(OUTPUT_DATA_DIR, name), sep=',')
        degrees = df['degree']
        counts = df['count']

        plt.plot(degrees, counts)
        plt.title(title)
        plt.xscale('log')
        plt.yscale('log')
        plt.savefig("{}/{}.png".format(FIG_DIR, name))
        plt.clf()


if __name__ == "__main__":
    main()
