import numpy as np

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib

FIG_DIR = 'figs/'
OUTPUT_DATA_DIR = 'output_data/'


def main():
    try:
        df = pd.read_csv(
            "configuration_model_output_data/min_contribution_5/projected_user/NumCommonNodes/strength_expected.csv",
            sep=',')
    except FileNotFoundError:
        return
    strength = df["strength"]
    expected = df["expected"]
    print(df)
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
    _, x_bins, y_bins = np.histogram2d(np.log10(expected + 1),
                                       np.log10(strength + 1),
                                       bins=[2 * x_bins.size, 2 * y_bins.size],
                                       weights=counts)

    print((strength / expected).min())
    print((strength / expected).argmin())
    print((strength / expected).max())
    print((strength / expected).argmax())

    plt.hist2d(expected,
               strength,
               weights=counts,
               bins=[10**x_bins, 10**y_bins],
               norm=matplotlib.colors.LogNorm())
    plt.colorbar()
    plt.title("strength vs expected (predicted)")
    plt.xscale('log')
    plt.yscale('log')
    plt.show()
    plt.clf()

if __name__ == "__main__":
    main()
