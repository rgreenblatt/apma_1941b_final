#!/usr/bin/env bash

mkdir -p figs_dots figs_dot_out
dot -Gdpi=300 -Tpng figs_dot/example_graph.dot -o figs_dot_out/example_graph.png
dot -Gdpi=300 -Tpng figs_dot/basic.dot -o figs_dot_out/basic.png
dot -Gdpi=300 -Tpng figs_dot/binary.dot -o figs_dot_out/binary.png
dot -Gdpi=300 -Tpng figs_dot/count.dot -o figs_dot_out/count.png
dot -Gdpi=300 -Tpng figs_dot/avg.dot -o figs_dot_out/avg.png
dot -Gdpi=300 -Tpng figs_dot/geometric_mean.dot -o figs_dot_out/geometric_mean.png
