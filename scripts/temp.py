import numpy as np

def main():
    p = np.array([0.001, 0.01, 0.003, 0.8238, 0.9, 0.9])

    actual_probs = p / (1 - (1 - p).prod())

    print(actual_probs)
    print(actual_probs.sum())

if __name__ == "__main__":
    main()

