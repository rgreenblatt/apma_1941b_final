import os

import pandas as pd
import matplotlib.pyplot as plt

FIG_DIR = 'figs/'
OUTPUT_DATA_DIR = 'output_data/'


def main():
    _, directories, _ = next(os.walk(OUTPUT_DATA_DIR))
    for dir_path in directories:
        print(dir_path)
        fig_dir = "{}/{}".format(FIG_DIR, dir_path)
        data_dir = "{}/{}".format(OUTPUT_DATA_DIR, dir_path)
        os.makedirs(FIG_DIR, exist_ok=True)

        for name, title in [
            ("user_degrees", "user degrees"), ("repo_degrees", "repo degrees"),
            ("user_total_contributions", "user total contributions"),
            ("repo_total_events", "repo total events"),
            ("user_projected_degrees", "user projected degrees"),
            ("repo_projected_degrees", "repo projected degrees"),
            ("user_projected_total_contributions",
             "user projected total contributions"),
            ("repo_projected_total_events", "repo projected total events")
        ]:

            try:
                df = pd.read_csv("{}/{}.csv".format(data_dir, name), sep=',')
            except FileNotFoundError:
                continue
            degrees = df['degree']
            counts = df['count']

            plt.plot(degrees, counts)
            plt.title(title)
            plt.xscale('log')
            plt.yscale('log')
            plt.savefig("{}/{}.png".format(fig_dir, name))
            plt.clf()


if __name__ == "__main__":
    main()
