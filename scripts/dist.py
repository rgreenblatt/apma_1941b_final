import numpy as np
import matplotlib.pyplot as plt
from scipy.optimize import curve_fit


def dist_up_to(n, beta, p_a, p_b):
    # not efficient
    Z = np.zeros((n, n, n))

    Z[0, 0, 0] = 1
    Z[1, 0, 0] = (1 - p_a - p_b)
    Z[1, 0, 1] = p_b
    Z[1, 1, 0] = p_a

    for i in range(2, n):
        b = beta(i)
        for j in range(i + 1):
            for k in range(i + 1 - j):
                prob_a_same_b_same = b * (i - 1 - j - k) / (i - 1) + (
                    1 - b) * (1 - p_a - p_b)
                Z[i, j, k] = Z[i - 1, j, k] * prob_a_same_b_same
                if j != 0:
                    prob_inc_a = b * (j - 1) / (i - 1) + (1 - b) * p_a
                    Z[i, j, k] += Z[i - 1, j - 1, k] * prob_inc_a
                if k != 0:
                    prob_inc_b = b * (k - 1) / (i - 1) + (1 - b) * p_b
                    Z[i, j, k] += Z[i - 1, j, k - 1] * prob_inc_b
    return Z


def main():
    n = 129
    p_a = 0.01
    p_b = 0.01
    Z = dist_up_to(n, lambda _: 0.0, p_a, p_b)

    fig = plt.figure()

    X = np.arange(0, 4, 1)
    Y = np.arange(0, 4, 1)

    X, Y = np.meshgrid(X, Y)

    ax = fig.add_subplot(2, 3, 1, projection='3d')
    ax.plot_surface(X, Y, Z[1, :4, :4])
    ax.set_title('Z[1]')

    ax = fig.add_subplot(2, 3, 2, projection='3d')
    ax.plot_surface(X, Y, Z[4, :4, :4])
    ax.set_title('Z[4]')

    ax = fig.add_subplot(2, 3, 3, projection='3d')
    ax.plot_surface(X, Y, Z[16, :4, :4])
    ax.set_title('Z[16]')

    X = np.arange(0, 32, 1)
    Y = np.arange(0, 32, 1)

    X, Y = np.meshgrid(X, Y)

    ax = fig.add_subplot(2, 3, 4, projection='3d')
    ax.plot_surface(X, Y, Z[16, :32, :32])
    ax.set_title('Z[16]')

    ax = fig.add_subplot(2, 3, 5, projection='3d')
    ax.plot_surface(X, Y, Z[32, :32, :32])
    ax.set_title('Z[32]')

    ax = fig.add_subplot(2, 3, 6, projection='3d')
    ax.plot_surface(X, Y, Z[128, :32, :32])
    ax.set_title('Z[128]')

    plt.tight_layout()

    # plt.show()
    plt.clf()

    def expectation_a(f):
        return (Z.sum(axis=-1) * f(np.array(range(n)))[None, :]).sum(axis=1)

    mean_a = expectation_a(lambda x: x)
    print(mean_a)

    X = np.arange(0, n, 1)
    Y = np.arange(0, n, 1)

    X, Y = np.meshgrid(X, Y)

    def expectation(Z, f):
        return (Z * f(X, Y)[None, :, :]).sum(axis=(-2, -1))

    value = expectation(Z, lambda a, b: np.sqrt(a * b))

    print(value)
    print(np.diff(value))

    plt.plot(value)
    plt.show()
    plt.clf()

    plt.plot(np.diff(value))

    def f(x, a, b, x_0):
        return np.sqrt(
            p_a * p_b) * (a * np.exp(-(x + x_0) * b) * np.sqrt((x + x_0) * b) +
                          (1 - np.exp(-(x + x_0) * b)))

    x = np.arange(0, n - 1, 1)
    p0 = [np.sqrt(p_a * p_b), np.sqrt(p_a * p_b), 1]
    popt, _ = curve_fit(f, x, np.diff(value), p0=p0, maxfev=100000)

    # print(popt)
    print(np.sqrt(p_a * p_b))

    plt.plot(x, f(x, *popt))

    plt.show()
    plt.clf()

    # n = 129

    # num = 10
    # p_a_vals = np.logspace(-1, -0.25, num=num)
    # values = np.zeros((num, n))
    # ratios = np.zeros((num))

    # for i, p_a in enumerate(p_a_vals):
    #     p_b = 0.1

    #     Z = dist_up_to(n, lambda _: 0.0, p_a, p_b)
    #     prob = Z[n-1, :5].sum() + Z[n-1, :, :5].sum()
    #     values[i, :] = expectation(Z, lambda a, b: np.sqrt(a * b)) / (
    #         prob * (p_a * p_b) + (1 - prob) * np.sqrt(p_a * p_b))
    #     ratios[i] = values[i, 128] / values[0, 128]
    #     # plt.plot(ratio_values)
    #     # plt.show()
    #     # plt.plot(values[i, j, :], label="p_a = {}, p_b = {}".format(p_a, p_b))

    # print(ratios)

    # plt.plot(p_a_vals, ratios)
    # plt.tight_layout()
    # plt.show()

    # # plt.legend()
    # # plt.tight_layout()
    # # plt.show()


if __name__ == "__main__":
    main()
