import os
import argparse

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib
from scipy import stats

FIG_DIR = 'figs/'
OUTPUT_DATA_DIR = 'output_data/'


def projected(legend, data_dirs, fig_dir):
    try:
        _, directories, _ = next(os.walk(data_dirs[0]))
    except StopIteration:
        return

    is_single = len(data_dirs) == 1

    for dir_path in directories:
        print(dir_path)
        connec_str_data_dirs = [
            "{}/{}".format(data_dir, dir_path) for data_dir in data_dirs
        ]
        connec_str_fig_dir = "{}/{}".format(fig_dir, dir_path)

        os.makedirs(connec_str_fig_dir, exist_ok=True)

        for name, header_name in [
            ("degrees", "degree"),
            ("expected", "strength"),
            ("strengths", "strength"),
            ("strengths_normalized", "strength"),
        ]:
            degrees_all = []
            counts_all = []
            try:
                for connec_str_data_dir in connec_str_data_dirs:
                    df = pd.read_csv("{}/{}.csv".format(
                        connec_str_data_dir, name),
                                     sep=',')

                    degrees_all.append(df[header_name])
                    counts_all.append(df['count'])
            except FileNotFoundError:
                continue

            for use_y_log in [False, True]:
                for degrees, counts in zip(degrees_all, counts_all):
                    if name == 'degrees' or (dir_path == "NumCommonNodes"
                                             and name == "strengths"):
                        plt.plot(degrees, counts / counts.sum())
                    else:
                        _, bins = np.histogram(np.log10(degrees + 1),
                                               weights=counts)

                        _, bins = np.histogram(np.log10(degrees + 1),
                                               bins=bins.size * 2,
                                               weights=counts)
                        bins = 10**bins

                        plt.hist(degrees,
                                 weights=counts,
                                 bins=bins,
                                 density=True,
                                 histtype='step')

                if not is_single:
                    plt.legend(legend)
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

        if not is_single:
            continue

        connec_str_data_dir = connec_str_data_dirs[0]

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
                   norm=matplotlib.colors.LogNorm(),
                   density=True)
        plt.colorbar()
        plt.xscale('log')
        plt.yscale('log')
        plt.savefig("{}/{}.png".format(connec_str_fig_dir,
                                       "strength_expected_scatter"))
        plt.clf()


def walklevel_exact(some_dir, level=1):
    some_dir = some_dir.rstrip(os.path.sep)
    assert os.path.isdir(some_dir)
    num_sep = some_dir.count(os.path.sep)
    for root, dirs, files in os.walk(some_dir):
        num_sep_this = root.count(os.path.sep)
        sub_level = num_sep_this - num_sep
        if sub_level == level:
            yield root, dirs, files
        if sub_level >= level:
            del dirs[:]


def contributions(data_dir, fig_dir):
    try:
        csv_files = os.listdir(data_dir)
    except FileNotFoundError:
        return
    for csv_file in csv_files:
        df = pd.read_csv("{}/{}".format(data_dir, csv_file), sep=',')

        nums = df['num']
        counts = df['count']

        plt.plot(nums, counts / counts.sum())
        plt.xscale('log')
        plt.yscale('log')
        plt.savefig("{}/{}.png".format(fig_dir, os.path.splitext(csv_file)[0]))
        plt.clf()


def make_figs(legend, data_dirs, fig_dir):
    os.makedirs(fig_dir, exist_ok=True)

    is_single = len(data_dirs) == 1

    if os.path.basename(fig_dir) == "contributions" and is_single:
        contributions(data_dirs[0], fig_dir)
        return

    for projected_dir in ["projected_repo", "projected_user"]:
        print(projected_dir)
        projected_data_dirs = [
            "{}/{}".format(data_dir, projected_dir) for data_dir in data_dirs
        ]
        projected_fig_dir = "{}/{}".format(fig_dir, projected_dir)
        projected(legend, projected_data_dirs, projected_fig_dir)

    for name in [
            "user_degrees",
            "repo_degrees",
            "user_total_contributions",
            "repo_total_events",
    ]:

        degrees_all = []
        counts_all = []
        try:
            for data_dir in data_dirs:
                df = pd.read_csv("{}/{}.csv".format(data_dir, name), sep=',')

                degrees_all.append(df['degree'])
                counts_all.append(df['count'])
        except FileNotFoundError:
            continue

        for degrees, counts in zip(degrees_all, counts_all):
            plt.plot(degrees, counts / counts.sum())

        if not is_single:
            plt.legend(legend)
        l = [counts.sum() for counts in counts_all]
        assert l.count(l[0]) == len(l)
        plt.xscale('log')
        plt.yscale('log')
        plt.savefig("{}/{}.png".format(fig_dir, name))
        plt.clf()

    if not is_single:
        return

    try:
        df = pd.read_csv("{}/{}.csv".format(data_dirs[0], "component_sizes"),
                         sep=',')
    except FileNotFoundError:
        return

    users, repos, counts = df['user_size'], df['repo_size'], df['count']

    loc = counts > 2
    users, repos, counts = users[loc], repos[loc], counts[loc]

    plt.hist2d(users,
               repos,
               weights=counts,
               bins=[users.max(), repos.max()],
               norm=matplotlib.colors.LogNorm(),
               density=True)
    plt.colorbar()
    plt.xlabel("users")
    plt.ylabel("repos")
    plt.savefig("{}/{}.png".format(fig_dir, "component_sizes"))
    plt.clf()


def main():
    parser = argparse.ArgumentParser(description='generate all figures')
    parser.add_argument('--fig-dir', default=FIG_DIR)
    parser.add_argument('--data-dir', default=OUTPUT_DATA_DIR)
    args = parser.parse_args()

    top_data_dir = args.data_dir.rstrip(os.path.sep) + os.path.sep
    for data_root, directories, _ in walklevel_exact(top_data_dir, 1):
        assert data_root[:len(top_data_dir)] == top_data_dir
        fig_root = args.fig_dir + data_root[len(top_data_dir):]
        for dir_path in directories:
            data_dir = os.path.join(data_root, dir_path)
            fig_dir = os.path.join(fig_root, dir_path)
            print("for", data_dir)
            make_figs([None], [data_dir], fig_dir)
            print()

    # TODO: fix this being hard coded???
    to_compare_legend = ['actual', 'random']
    compare_name = "actual_vs_configuration"
    to_compare_dirs = ['actual_graph', 'configuration_model']

    path = "{}/{}".format(args.data_dir, to_compare_dirs[0])

    for sub in os.listdir(path):
        data_dirs = [
            "{}/{}/{}".format(args.data_dir, to_c, sub)
            for to_c in to_compare_dirs
        ]
        fig_dir = "{}/{}/{}".format(args.fig_dir, compare_name, sub)

        print("for", data_dirs)
        make_figs(to_compare_legend, data_dirs, fig_dir)


if __name__ == "__main__":
    main()
