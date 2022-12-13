#!/usr/bin/env python3

# Ad hoc utility to plot the aspect ratio cumulative histogram
# of a set of bounding boxes as reported by blaise.
#
# Exercised on the 315k dataset:
#
# $ misc/plot_ar_hist.py misc/out/mbari_training_data_315k_bounding_boxes.csv \
#                        misc/out/mbari_training_data_315k_bounding_boxes_hist.png
#
# where mbari_training_data_315k_bounding_boxes.csv was generated by blaise on the dev-box:
#
# $ blaise --yolo mbari_training_data_315k/images mbari_training_data_315k/labels mbari_training_data_315k/yolo.names \
#          --bb-info mbari_training_data_315k_bounding_boxes.csv

import matplotlib.pyplot as plt
import pandas as pd


def plot_ar_hist(in_file, out_file=None):
    data = pd.read_csv(in_file, sep=',')
    what = 'aspect_ratio'
    plt.xlim(0.9, 3)
    aspect_ratio = data[what]
    num = len(aspect_ratio)
    # ignore any `NaN` or `inf` values: (there are 27 such cases in the 315k dataset)
    with pd.option_context('use_inf_as_na', True):
        aspect_ratio.dropna(how="all", inplace=True)
        dropped = num - len(aspect_ratio)
        print(f"dropped {dropped:,} out of {num:,} NaN or inf aspect_ratio values")

    aspect_ratio.plot(kind='hist', bins=1000, cumulative=True, histtype='step')
    plt.xlabel('AR')
    plt.ylabel('Count')
    plt.title(f'{what} cumulative distribution')
    plt.grid(True)
    if out_file:
        plt.savefig(out_file, bbox_inches='tight')
    else:
        plt.show()

if __name__ == "__main__":
    import sys

    if len(sys.argv) > 1:
        in_file = sys.argv[1]
        out_file = sys.argv[2] if len(sys.argv) > 2 else None
        plot_ar_hist(in_file, out_file)
    else:
        print(f'Usage: {sys.argv[0]} plot_ar_hist.py <file.csv> [<file.png>]')
        print(f'   eg: {sys.argv[0]} plot_ar_hist.py bounding_boxes.csv bounding_boxes_hist.png')
