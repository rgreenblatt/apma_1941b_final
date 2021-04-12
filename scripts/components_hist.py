import os

import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from matplotlib import pylab

FIG_DIR = 'figs/'
OUTPUT_DATA_DIR = 'output_data/'


def main():
    for dir_path in os.listdir(OUTPUT_DATA_DIR):
        fig_dir = "{}/{}".format(FIG_DIR, dir_path)
        data_dir = "{}/{}".format(OUTPUT_DATA_DIR, dir_path)
        os.makedirs(fig_dir, exist_ok=True)

        df = pd.read_csv("{}/{}.csv".format(data_dir, "component_sizes"),
                         sep=',')

        users, repos, counts = df['user_size'], df['repo_size'], df['count']

        loc = counts > 2
        users, repos, counts = users[loc], repos[loc], counts[loc]

        pylab.hist2d(users,
                     repos,
                     weights=np.log(counts),
                     bins=[users.max(), repos.max()])
        pylab.title("component sizes")
        pylab.xlabel("users")
        pylab.ylabel("repos")
        plt.savefig("{}/{}.png".format(fig_dir, "component_sizes"))
        plt.clf()


if __name__ == "__main__":
    main()
