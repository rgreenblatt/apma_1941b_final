import os
import argparse

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib

FIG_DIR = 'figs/'
OUTPUT_DATA_DIR = 'output_data/'


def projected(data_dir, fig_dir):
    try:
        _, directories, _ = next(os.walk(data_dir))
    except StopIteration:
        return

    for dir_path in directories:
        print(dir_path)
        connec_str_data_dir = "{}/{}".format(data_dir, dir_path)
        connec_str_fig_dir = "{}/{}".format(fig_dir, dir_path)

        os.makedirs(connec_str_fig_dir, exist_ok=True)

        for name, title, header_name in [
            ("degrees", "degrees", "degree"),
            ("expected", "expected", "strength"),
            ("strengths", "strengths", "strength"),
            ("strengths_normalized", "strengths_normalized", "strength"),
        ]:
            try:
                df = pd.read_csv("{}/{}.csv".format(connec_str_data_dir, name),
                                 sep=',')
            except FileNotFoundError:
                continue

            degrees = df[header_name]
            counts = df['count']

            for use_y_log in [False, True]:
                if header_name == 'degrees':
                    plt.plot(degrees, counts)
                else:
                    _, bins = np.histogram(np.log10(degrees + 1),
                                           weights=counts)
                    _, bins = np.histogram(np.log10(degrees + 1),
                                           bins=bins.size * 2,
                                           weights=counts)
                    plt.hist(degrees,
                             weights=counts,
                             bins=10**bins,
                             density=True)
                plt.title(title)
                plt.xscale('log')
                if use_y_log:
                    plt.yscale('log')
                    actual_name = name
                else:
                    plt.yscale('linear')
                    actual_name = name + "_no_y_log"

                plt.savefig("{}/{}.png".format(connec_str_fig_dir,
                                               actual_name))
                plt.clf()

        try:
            df = pd.read_csv("{}/{}.csv".format(connec_str_data_dir,
                                                "strength_expected"),
                             sep=',')
        except FileNotFoundError:
            continue
        strength = df["strength"]
        expected = df["expected"]
        counts = df['count']

        total = counts.sum()

        mean_str = (strength * counts).sum() / total
        mean_sqr_str = (strength**2 * counts).sum() / total
        mean_expected = (expected * counts).sum() / total
        mean_sqr_expected = (expected**2 * counts).sum() / total

        mean_str_expected = (strength * expected * counts).sum() / total

        correlation = (mean_str_expected - mean_str * mean_expected) / (
            np.sqrt(mean_sqr_str - mean_str**2) *
            np.sqrt(mean_sqr_expected - mean_expected**2))

        print("correlation between strength and expected is: ", correlation)

        _, x_bins, y_bins = np.histogram2d(np.log10(expected + 1),
                                           np.log10(strength + 1),
                                           weights=counts)
        _, x_bins, y_bins = np.histogram2d(
            np.log10(expected + 1),
            np.log10(strength + 1),
            bins=[2 * x_bins.size, 2 * y_bins.size],
            weights=counts)

        plt.hist2d(expected,
                   strength,
                   weights=counts,
                   bins=[10**x_bins, 10**y_bins],
                   norm=matplotlib.colors.LogNorm())
        plt.colorbar()
        plt.title("strength vs expected (predicted)")
        plt.xscale('log')
        plt.yscale('log')
        plt.savefig("{}/{}.png".format(connec_str_fig_dir,
                                       "strength_expected_scatter"))
        plt.clf()


def main():
    parser = argparse.ArgumentParser(
        description='plot a histogram of the component sizes')
    parser.add_argument('--fig-dir', default=FIG_DIR)
    parser.add_argument('--data-dir', default=OUTPUT_DATA_DIR)
    args = parser.parse_args()

    _, directories, _ = next(os.walk(args.data_dir))
    for dir_path in directories:
        print(dir_path)
        fig_dir = "{}/{}".format(args.fig_dir, dir_path)
        data_dir = "{}/{}".format(args.data_dir, dir_path)
        os.makedirs(fig_dir, exist_ok=True)

        for projected_dir in ["projected_repo", "projected_user"]:
            print(projected_dir)
            projected_data_dir = "{}/{}".format(data_dir, projected_dir)
            projected_fig_dir = "{}/{}".format(fig_dir, projected_dir)
            projected(projected_data_dir, projected_fig_dir)

        for name, title in [
            ("user_degrees", "user degrees"),
            ("repo_degrees", "repo degrees"),
            ("user_total_contributions", "user total contributions"),
            ("repo_total_events", "repo total events"),
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

        try:
            df = pd.read_csv("{}/{}.csv".format(data_dir, "component_sizes"),
                             sep=',')
        except FileNotFoundError:
            continue

        users, repos, counts = df['user_size'], df['repo_size'], df['count']

        loc = counts > 2
        users, repos, counts = users[loc], repos[loc], counts[loc]

        plt.hist2d(users,
                   repos,
                   weights=counts,
                   bins=[users.max(), repos.max()],
                   norm=matplotlib.colors.LogNorm())
        plt.title("component sizes")
        plt.xlabel("users")
        plt.ylabel("repos")
        plt.savefig("{}/{}.png".format(fig_dir, "component_sizes"))
        plt.clf()


if __name__ == "__main__":
    main()
