2022-12

- added `--max-ar <value>` to consider bounding boxes with at most the given aspect ratio
- added `--bb-info <csv-file>` to report size, aspect ratio of loaded bounding boxes
- added `--resize <width> <height>` option.
  This uses `resize_exact` (thus not preserving aspect ratio).
  Eg.,: `just run -p data -o data/out --resize 256 256`

- added `--yolo` option
  Initial tests with a small sample from the 315K dataset (not included in this repo):
  ```shell
  just rrun -y data/mbari_training_data_315k_SAMPLE/images data/mbari_training_data_315k_SAMPLE/labels data/mbari_training_data_315k_SAMPLE/yolo.names -o data/mbari_training_data_315k_SAMPLE/out
  ``` 
- refactoring toward enabling other annotation formats

2022-11

- some general adjustments;  new --summary option
- initial version
  - annotation xml parsing
  - image cropping
  - multithreaded dispatch
