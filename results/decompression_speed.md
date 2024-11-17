# Decompression Speed

The following tests aim to measure the performance of various dictionary variants:

1) **Variant 1**: Stores token IDs using classic Variable Byte Encoding (VBE) without imposing a maximum entry length in the dictionary.

2) **Variant 2**: Similar to the first, but with a 16-byte limit on dictionary entries. This allows the use of `mm_loadu_si128` and `_mm_storeu_si128` operations to efficiently transfer data from the dictionary to the buffer.

3) **Variant 3**: Saves continuation bits in a separate bitvector to decode 8 bits in parallel using SIMD instructions. Dictionary entries have a maximum length of 16 bytes.

4) **Variant 4**: Explicitly saves token IDs using 2 bytes, with a maximum entry length of 16 bytes in the dictionary.

## Variant 1: VBE token IDs, no limit to dictionary entries.

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| c_name      | 4,941 | 2178,81 | 1,72 | 0,31 | 0,03 | 0,01 |
| chinese     | 2,203 |  555,18 | 0,70 | 0,28 | 0,02 | 0,02 |
| city        | 1,673 |  325,36 | 0,12 | 0,06 | 0,00 | 0,00 |
| credentials | 1,941 |  372,33 | 0,13 | 0,06 | 0,00 | 0,00 |
| email       | 2,192 |  507,14 | 2,02 | 0,85 | 0,04 | 0,03 |
| faust       | 1,830 |  390,62 | 0,29 | 0,14 | 0,01 | 0,01 |
| firstname   | 1,551 |  322,74 | 0,36 | 0,22 | 0,01 | 0,01 |
| genome      | 2,432 |  601,18 | 0,86 | 0,32 | 0,02 | 0,01 |
| hamlet      | 2,522 |  478,70 | 0,26 | 0,09 | 0,01 | 0,01 |
| hex         | 1,405 |  392,71 | 0,76 | 0,51 | 0,01 | 0,01 |
| japanese    | 2,390 |  522,48 | 0,19 | 0,07 | 0,01 | 0,01 |
| l_comment   | 4,279 |  890,68 | 2,50 | 0,52 | 0,05 | 0,02 |
| lastname    | 1,855 |  441,62 | 2,15 | 1,08 | 0,05 | 0,04 |
| location    | 2,204 |  474,99 | 2,52 | 1,07 | 0,04 | 0,03 |
| movies      | 1,761 |  420,46 | 1,93 | 1,00 | 0,05 | 0,05 |
| ps_comment  | 6,048 | 1295,10 | 2,35 | 0,32 | 0,05 | 0,01 |
| street      | 2,067 |  388,32 | 0,12 | 0,05 | 0,00 | 0,00 |
| urls        | 5,235 | 1421,71 | 5,94 | 0,93 | 0,16 | 0,05 |
| urls2       | 2,413 |  558,69 | 1,57 | 0,58 | 0,04 | 0,03 |
| uuid        | 2,198 |  561,75 | 3,43 | 1,47 | 0,05 | 0,04 |
| wiki        | 1,914 |  422,10 | 2,14 | 1,00 | 0,07 | 0,05 |
| wikipedia   | 2,358 |  549,11 | 2,71 | 1,02 | 0,08 | 0,05 |
| yago        | 1,769 |  402,84 | 1,74 | 0,89 | 0,05 | 0,04 |
| **Average** | 2,573 | 629,33 | | | | | 

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    | 3,212 |  798,09 | 12,21 | 3,41 | 0,27 | 0,12 |
| amazon_beauty_reviews   | 3,360 |  874,07 | 16,04 | 4,38 | 0,27 | 0,12 |
| amazon_handmade_titles  | 3,141 |  729,42 | 16,05 | 4,71 | 0,28 | 0,12 |
| amazon_handmade_reviews | 3,463 |  907,70 | 16,02 | 4,22 | 0,28 | 0,12 |
| ebay_titles             | 3,156 |  870,27 |  1,69 | 0,42 | 0,08 | 0,03 |
| flipkart_titles         | 2,821 |  787,24 |  0,77 | 0,21 | 0,04 | 0,02 |
| food_com_descriptions   | 3,529 |  896,92 | 16,02 | 4,13 | 0,29 | 0,12 |
| food_com_reviews        | 3,484 |  872,24 | 16,02 | 4,19 | 0,28 | 0,12 |
| food_com_titles         | 3,406 |  989,77 |  6,17 | 1,58 | 0,16 | 0,07 |
| goodreads_descriptions  | 2,534 |  650,19 | 16,05 | 6,01 | 0,20 | 0,12 |
| google_reviews          | 3,090 |  771,63 | 16,00 | 4,81 | 0,25 | 0,12 |
| huffpost_descriptions   | 2,980 |  720,10 | 16,04 | 5,03 | 0,23 | 0,12 |
| huffpost_headlines      | 2,690 |  691,14 | 11,69 | 4,00 | 0,22 | 0,12 |
| imdb_reviews            | 3,055 |  781,16 | 16,05 | 4,90 | 0,23 | 0,12 |
| linkedin_descriptions   | 3,496 |  899,07 | 16,04 | 4,19 | 0,27 | 0,12 |
| linkedin_job_postings   | 3,859 | 1013,97 | 16,04 | 3,67 | 0,38 | 0,11 |
| myntra_titles           | 3,785 | 1103,31 |  0,63 | 0,12 | 0,03 | 0,01 |
| quora_questions         | 3,450 |  692,74 | 16,05 | 4,25 | 0,28 | 0,12 |
| reddit_posts            | 3,000 |  773,21 | 16,01 | 4,97 | 0,24 | 0,12 |
| reddit_titles           | 3,243 |  778,65 | 16,05 | 4,58 | 0,25 | 0,12 |
| walmart_titles          | 2,029 |  563,91 |  1,94 | 0,84 | 0,07 | 0,05 |
| wikipedia_movie_plots   | 3,009 |  767,84 | 16,02 | 4,97 | 0,23 | 0,12 |
| youtube_comments        | 2,699 |  679,98 | 16,01 | 5,59 | 0,22 | 0,12 |
| youtube_titles          | 2,929 |  973,32 |  1,91 | 0,47 | 0,13 | 0,05 |
| **Average**             | 3,143 |  816,08 | | | | | 

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    | 3,212 | 705,96 |  12,21 |   3,41 | 0,27 | 0,12 |
| amazon_beauty_reviews   | 3,595 | 776,25 | 116,18 |  31,92 | 0,27 | 0,12 |
| amazon_handmade_titles  | 3,146 | 689,52 |  16,37 |   4,81 | 0,28 | 0,12 |
| amazon_handmade_reviews | 3,739 | 798,73 |  86,09 |  22,62 | 0,28 | 0,12 |
| ebay_titles             | 3,156 | 791,25 |   1,69 |   0,42 | 0,08 | 0,03 |
| flipkart_titles         | 2,821 | 734,90 |   0,77 |   0,21 | 0,04 | 0,02 |
| food_com_descriptions   | 3,714 | 798,05 |  43,19 |  11,22 | 0,28 | 0,12 |
| food_com_reviews        | 3,768 | 655,37 | 300,51 |  79,34 | 0,28 | 0,12 |
| food_com_titles         | 3,406 | 895,28 |   6,17 |   1,58 | 0,16 | 0,07 |
| google_reviews          | 3,402 | 729,49 |  95,94 |  27,83 | 0,25 | 0,12 |
| huffpost_descriptions   | 3,023 | 663,95 |  22,89 |   7,21 | 0,23 | 0,12 |
| huffpost_headlines      | 2,690 | 630,61 |  11,69 |   4,00 | 0,22 | 0,12 |
| imdb_reviews            | 3,201 | 684,49 |  62,45 |  19,15 | 0,23 | 0,12 |
| linkedin_descriptions   | 3,538 | 791,71 |  19,16 |   5,02 | 0,27 | 0,12 |
| linkedin_job_postings   | 4,109 | 716,60 | 445,90 | 108,04 | 0,36 | 0,12 |
| myntra_titles           | 3,785 | 931,66 |   0,63 |   0,12 | 0,03 | 0,01 |
| quora_questions         | 3,534 | 626,84 |  22,97 |   6,10 | 0,28 | 0,12 |
| reddit_posts            | 2,985 | 564,88 | 194,19 |  64,69 | 0,23 | 0,12 |
| reddit_titles           | 3,276 | 694,52 |  32,04 |   9,41 | 0,25 | 0,12 |
| walmart_titles          | 2,029 | 504,88 |   1,94 |   0,84 | 0,07 | 0,05 |
| wikipedia_movie_plots   | 3,079 | 676,79 |  71,99 |  23,03 | 0,22 | 0,12 |
| youtube_comments        | 2,764 | 609,54 |  56,23 |  20,00 | 0,22 | 0,12 |
| youtube_titles          | 2,929 | 884,25 |   1,91 |   0,47 | 0,13 | 0,05 |
| **Average**             | 3,257 | 719,81 | | | | | 

## Variant 2: VBE token IDs, dictionary entries up to 16 bytes.

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| c_name      | 4,941 | 4458,15 | 1,72 | 0,31 | 0,03 | 0,01 |
| chinese     | 2,203 | 1764,75 | 0,70 | 0,28 | 0,02 | 0,02 |
| city        | 1,673 |  873,98 | 0,12 | 0,06 | 0,00 | 0,00 |
| credentials | 1,941 | 1041,07 | 0,13 | 0,06 | 0,00 | 0,00 |
| email       | 2,192 | 1707,27 | 2,02 | 0,85 | 0,04 | 0,03 |
| faust       | 1,830 | 1194,09 | 0,29 | 0,14 | 0,01 | 0,01 |
| firstname   | 1,551 |  847,99 | 0,36 | 0,22 | 0,01 | 0,01 |
| genome      | 2,432 | 1142,21 | 0,86 | 0,32 | 0,02 | 0,01 |
| hamlet      | 2,483 | 1435,96 | 0,26 | 0,09 | 0,01 | 0,01 |
| hex         | 1,405 |  754,86 | 0,76 | 0,51 | 0,01 | 0,01 |
| japanese    | 2,383 | 1488,16 | 0,19 | 0,07 | 0,01 | 0,01 |
| l_comment   | 4,290 | 3056,85 | 2,50 | 0,52 | 0,04 | 0,02 |
| lastname    | 1,844 | 1553,06 | 2,15 | 1,09 | 0,04 | 0,04 |
| location    | 2,204 | 1593,33 | 2,52 | 1,07 | 0,04 | 0,03 |
| movies      | 1,761 | 1513,41 | 1,93 | 1,00 | 0,05 | 0,05 |
| ps_comment  | 6,267 | 3242,35 | 2,35 | 0,34 | 0,03 | 0,01 |
| street      | 2,067 | 1118,96 | 0,12 | 0,05 | 0,00 | 0,00 |
| urls        | 4,377 | 3259,91 | 5,94 | 1,24 | 0,07 | 0,05 |
| urls2       | 2,298 | 1510,56 | 1,57 | 0,63 | 0,03 | 0,02 |
| uuid        | 2,198 | 1451,92 | 3,43 | 1,47 | 0,05 | 0,04 |
| wiki        | 1,896 | 1279,81 | 2,14 | 1,02 | 0,06 | 0,05 |
| wikipedia   | 2,355 | 2311,93 | 2,71 | 1,02 | 0,08 | 0,05 |
| yago        | 1,765 | 1483,72 | 1,74 | 0,89 | 0,05 | 0,04 |
| **Average** | 2,537 | 1742,80 | | | | | 

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    | 3,162 | 1573,75 | 12,21 | 3,51 | 0,23 | 0,12 |
| amazon_beauty_reviews   | 3,348 | 1689,84 | 16,04 | 4,41 | 0,25 | 0,12 |
| amazon_handmade_titles  | 3,086 | 1559,04 | 16,05 | 4,85 | 0,23 | 0,12 |
| amazon_handmade_reviews | 3,432 | 1670,00 | 16,02 | 4,28 | 0,26 | 0,12 |
| ebay_titles             | 3,028 | 2248,12 |  1,69 | 0,48 | 0,05 | 0,03 |
| flipkart_titles         | 2,672 | 1902,17 |  0,77 | 0,24 | 0,03 | 0,02 |
| food_com_descriptions   | 3,495 | 1703,77 | 16,02 | 4,20 | 0,26 | 0,12 |
| food_com_reviews        | 3,446 | 1713,88 | 16,02 | 4,26 | 0,26 | 0,12 |
| food_com_titles         | 3,382 | 2890,00 |  6,17 | 1,62 | 0,14 | 0,07 |
| goodreads_descriptions  | 2,532 | 1350,55 | 16,05 | 6,01 | 0,20 | 0,12 |
| google_reviews          | 3,071 | 1557,43 | 16,00 | 4,85 | 0,24 | 0,12 |
| huffpost_descriptions   | 2,969 | 2344,12 | 16,04 | 5,05 | 0,22 | 0,12 |
| huffpost_headlines      | 2,682 | 2145,58 | 11,69 | 4,02 | 0,21 | 0,12 |
| imdb_reviews            | 3,048 | 1551,88 | 16,05 | 4,91 | 0,23 | 0,12 |
| linkedin_descriptions   | 3,455 | 1697,66 | 16,04 | 4,27 | 0,25 | 0,12 |
| linkedin_job_postings   | 3,714 | 1812,27 | 16,04 | 3,93 | 0,26 | 0,12 |
| myntra_titles           | 3,710 | 2397,33 |  0,63 | 0,14 | 0,02 | 0,01 |
| quora_questions         | 3,346 | 1603,26 | 16,05 | 4,44 | 0,23 | 0,12 |
| reddit_posts            | 2,978 | 1563,73 | 16,01 | 5,02 | 0,23 | 0,12 |
| reddit_titles           | 3,187 | 1562,50 | 16,05 | 4,69 | 0,23 | 0,12 |
| walmart_titles          | 2,026 | 1697,86 |  1,94 | 0,84 | 0,07 | 0,05 |
| wikipedia_movie_plots   | 3,005 | 1559,23 | 16,02 | 4,98 | 0,22 | 0,12 |
| youtube_comments        | 2,688 | 1404,57 | 16,01 | 5,62 | 0,21 | 0,12 |
| youtube_titles          | 2,548 | 2678,76 |  1,91 | 0,59 | 0,11 | 0,06 |
| **Average**             | 3,084 | 1828,22 | | | | | 

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | 
| amazon_beauty_titles    | 3,162 | 2751,03 |  12,21 |   3,51 | 0,23 | 0,12 |
| amazon_beauty_reviews   | 3,573 | 2845,42 | 116,18 |  32,14 | 0,25 | 0,12 |
| amazon_handmade_titles  | 3,091 | 2595,59 |  16,37 |   4,94 | 0,23 | 0,12 |
| amazon_handmade_reviews | 3,686 | 2977,70 |  86,09 |  22,98 | 0,26 | 0,12 |
| ebay_titles             | 3,028 | 2569,61 |   1,69 |   0,48 | 0,05 | 0,03 |
| flipkart_titles         | 2,672 | 1974,69 |   0,77 |   0,24 | 0,03 | 0,02 |
| food_com_descriptions   | 3,665 | 2943,37 |  43,19 |  11,40 | 0,26 | 0,12 |
| food_com_reviews        | 3,709 | 1790,03 | 300,51 |  80,65 | 0,26 | 0,12 |
| food_com_titles         | 3,382 | 3416,87 |   6,17 |   1,62 | 0,14 | 0,07 |
| google_reviews          | 3,377 | 2777,35 |  95,94 |  28,05 | 0,24 | 0,12 |
| huffpost_descriptions   | 3,010 | 2448,99 |  22,89 |   7,26 | 0,22 | 0,12 |
| huffpost_headlines      | 2,682 | 2315,28 |  11,69 |   4,02 | 0,21 | 0,12 |
| imdb_reviews            | 3,191 | 2599,12 |  62,45 |  19,22 | 0,23 | 0,12 |
| linkedin_descriptions   | 3,494 | 2942,08 |  19,16 |   5,11 | 0,25 | 0,12 |
| linkedin_job_postings   | 3,941 | 1884,04 | 445,90 | 112,76 | 0,26 | 0,12 |
| myntra_titles           | 3,710 | 2690,34 |   0,63 |   0,14 | 0,02 | 0,01 |
| quora_questions         | 3,417 | 1645,31 |  22,97 |   6,36 | 0,23 | 0,12 |
| reddit_posts            | 2,955 | 1568,12 | 194,19 |  65,36 | 0,22 | 0,12 |
| reddit_titles           | 3,218 | 2558,15 |  32,04 |   9,60 | 0,23 | 0,12 |
| walmart_titles          | 2,026 | 1996,70 |   1,94 |   0,84 | 0,07 | 0,05 |
| wikipedia_movie_plots   | 3,073 | 2497,43 |  71,99 |  23,08 | 0,22 | 0,12 |
| youtube_comments        | 2,752 | 2345,57 |  56,23 |  20,11 | 0,20 | 0,12 |
| youtube_titles          | 2,548 | 2997,03 |   1,91 |   0,59 | 0,11 | 0,06 |
| **Average**             | 3,190 | 2483,91 | | | | | 

## Variant 3: VBE token IDs with continuation bits stored separately and decoded in batches of 8 using simd, dictionary entries up to 16 bytes.

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Continuation Bits (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| c_name      | 4.982 | 5143.55 | 1.72 | 0.29 | 0.00 | 0.03 | 0.01 |
| chinese     | 2.177 | 2861.78 | 0.70 | 0.26 | 0.00 | 0.02 | 0.02 |
| city        | 1.724 | 1243.15 | 0.12 | 0.05 | 0.00 | 0.00 | 0.00 |
| credentials | 1.994 | 2445.59 | 0.13 | 0.05 | 0.00 | 0.00 | 0.00 |
| email       | 2.177 | 2576.56 | 2.02 | 0.79 | 0.00 | 0.04 | 0.03 |
| faust       | 1.850 | 2344.25 | 0.29 | 0.13 | 0.00 | 0.01 | 0.01 |
| firstname   | 1.573 | 1883.42 | 0.36 | 0.20 | 0.00 | 0.01 | 0.01 |
| genome      | 2.481 | 1852.88 | 0.86 | 0.29 | 0.00 | 0.02 | 0.01 |
| hamlet      | 2.518 | 3130.11 | 0.26 | 0.08 | 0.00 | 0.01 | 0.01 |
| hex         | 1.471 | 1621.41 | 0.76 | 0.45 | 0.00 | 0.01 | 0.01 |
| japanese    | 2.425 | 3073.07 | 0.19 | 0.06 | 0.00 | 0.01 | 0.01 |
| l_comment   | 4.302 | 5123.35 | 2.50 | 0.48 | 0.00 | 0.04 | 0.02 |
| lastname    | 1.829 | 2159.03 | 2.15 | 1.02 | 0.00 | 0.04 | 0.04 |
| location    | 2.200 | 2569.37 | 2.52 | 1.00 | 0.00 | 0.04 | 0.03 |
| movies      | 1.733 | 2199.38 | 1.93 | 0.94 | 0.00 | 0.05 | 0.05 |
| ps_comment  | 6.198 | 6385.64 | 2.35 | 0.31 | 0.00 | 0.03 | 0.01 |
| street      | 2.125 | 2528.40 | 0.12 | 0.04 | 0.00 | 0.00 | 0.00 |
| urls        | 4.308 | 4697.65 | 5.94 | 1.17 | 0.00 | 0.07 | 0.05 |
| urls2       | 2.293 | 2708.59 | 1.57 | 0.58 | 0.00 | 0.03 | 0.02 |
| uuid        | 2.244 | 2427.43 | 3.43 | 1.33 | 0.00 | 0.05 | 0.04 |
| wiki        | 1.869 | 1443.84 | 2.14 | 0.96 | 0.00 | 0.06 | 0.05 |
| wikipedia   | 2.315 | 2932.87 | 2.71 | 0.97 | 0.00 | 0.08 | 0.05 |
| yago        | 1.743 | 2196.71 | 1.74 | 0.84 | 0.00 | 0.05 | 0.04 |
| **Average** | 2,545 | 2849,91 | | | | | |

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Continuation Bits (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    | 3.092 | 2763.16 | 12.21 | 3.30 | 0.00 | 0.27 | 0.14 |
| amazon_beauty_reviews   | 3.261 | 2831.48 | 16.04 | 4.16 | 0.00 | 0.32 | 0.15 |
| amazon_handmade_titles  | 3.096 | 2579.59 | 16.05 | 4.28 | 0.00 | 0.41 | 0.20 |
| amazon_handmade_reviews | 3.341 | 2942.83 | 16.02 | 4.06 | 0.00 | 0.31 | 0.15 |
| ebay_titles             | 3.007 | 3767.90 |  1.69 | 0.45 | 0.00 | 0.05 | 0.03 |
| flipkart_titles         | 2.666 | 3651.43 |  0.77 | 0.22 | 0.00 | 0.03 | 0.02 |
| food_com_descriptions   | 3.397 | 2943.47 | 16.02 | 3.98 | 0.00 | 0.32 | 0.15 |
| food_com_reviews        | 3.353 | 2620.45 | 16.02 | 4.03 | 0.00 | 0.32 | 0.15 |
| food_com_titles         | 3.307 | 4066.21 |  6.17 | 1.55 | 0.00 | 0.14 | 0.07 |
| goodreads_descriptions  | 2.532 | 2102.20 | 16.05 | 5.38 | 0.01 | 0.37 | 0.21 |
| google_reviews          | 3.011 | 2613.96 | 16.00 | 4.49 | 0.00 | 0.34 | 0.17 |
| huffpost_descriptions   | 2.931 | 2396.74 | 16.04 | 4.60 | 0.00 | 0.37 | 0.19 |
| huffpost_headlines      | 2.642 | 2329.30 | 11.69 | 3.69 | 0.00 | 0.31 | 0.17 |
| imdb_reviews            | 3.000 | 2571.14 | 16.05 | 4.51 | 0.00 | 0.35 | 0.18 |
| linkedin_descriptions   | 3.372 | 2958.01 | 16.04 | 4.01 | 0.00 | 0.32 | 0.15 |
| linkedin_job_postings   | 3.635 | 3242.73 | 16.04 | 3.65 | 0.00 | 0.35 | 0.16 |
| myntra_titles           | 3.731 | 4938.75 |  0.63 | 0.13 | 0.00 | 0.02 | 0.01 |
| quora_questions         | 3.294 | 1745.66 | 16.05 | 4.08 | 0.00 | 0.34 | 0.17 |
| reddit_posts            | 2.936 | 2671.95 | 16.01 | 4.61 | 0.00 | 0.35 | 0.18 |
| reddit_titles           | 3.136 | 2683.59 | 16.05 | 4.31 | 0.00 | 0.34 | 0.17 |
| walmart_titles          | 1.988 | 2595.72 |  1.94 | 0.80 | 0.00 | 0.07 | 0.05 |
| wikipedia_movie_plots   | 2.975 | 2529.45 | 16.02 | 4.52 | 0.00 | 0.37 | 0.19 |
| youtube_comments        | 2.694 | 2358.63 | 16.01 | 5.00 | 0.01 | 0.39 | 0.21 |
| youtube_titles          | 2.484 | 3420.59 |  1.91 | 0.57 | 0.00 | 0.11 | 0.06 |
| **Average**             | 3,037 | 2888,54 | | | | | |

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Continuation Bits (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    | 3,092 | 2910,68 |  12,21 |  3,30 | 0,00 | 0,27 | 0,14 |
| amazon_beauty_reviews   | 3,692 | 2829,23 | 116,18 | 28,71 | 0,03 | 0,56 | 0,25 |
| amazon_handmade_titles  | 3,104 | 2517,85 |  16,37 |  4,36 | 0,00 | 0,41 | 0,21 |
| amazon_handmade_reviews | 3,786 | 2957,74 |  86,09 | 20,53 | 0,02 | 0,57 | 0,25 |
| ebay_titles             | 3,007 | 3804,01 |   1,69 |  0,45 | 0,00 | 0,05 | 0,03 |
| flipkart_titles         | 2,666 | 3598,78 |   0,77 |  0,22 | 0,00 | 0,03 | 0,02 |
| food_com_descriptions   | 3,701 | 2923,33 |  43,19 | 10,15 | 0,01 | 0,58 | 0,25 |
| food_com_reviews        | 3,873 | 1786,63 | 300,51 | 71,90 | 0,08 | 0,57 | 0,25 |
| food_com_titles         | 3,307 | 3952,23 |   6,17 |  1,55 | 0,00 | 0,14 | 0,07 |
| google_reviews          | 3,510 | 2694,65 |  95,94 | 24,86 | 0,03 | 0,53 | 0,25 |
| huffpost_descriptions   | 3,031 | 2492,84 |  22,89 |  6,37 | 0,01 | 0,50 | 0,25 |
| huffpost_headlines      | 2,642 | 2371,73 |  11,69 |  3,69 | 0,00 | 0,31 | 0,17 |
| imdb_reviews            | 3,311 | 2586,31 |  62,45 | 16,95 | 0,02 | 0,51 | 0,25 |
| linkedin_descriptions   | 3,432 | 3022,56 |  19,16 |  4,72 | 0,01 | 0,37 | 0,18 |
| linkedin_job_postings   | 4,187 | 1887,40 | 445,90 | 98,98 | 0,10 | 0,57 | 0,25 |
| myntra_titles           | 3,731 | 5098,36 |   0,63 |  0,13 | 0,00 | 0,02 | 0,01 |
| quora_questions         | 3,430 | 1693,95 |  22,97 |  5,63 | 0,01 | 0,46 | 0,22 |
| reddit_posts            | 3,104 | 1626,90 | 194,19 | 57,90 | 0,06 | 0,48 | 0,25 |
| reddit_titles           | 3,281 | 2651,75 |  32,04 |  8,44 | 0,01 | 0,50 | 0,25 |
| walmart_titles          | 1,988 | 2530,91 |   1,94 |  0,80 | 0,00 | 0,07 | 0,05 |
| wikipedia_movie_plots   | 3,216 | 2540,56 |  71,99 | 20,27 | 0,02 | 0,49 | 0,25 |
| youtube_comments        | 2,875 | 2285,27 |  56,23 | 17,65 | 0,02 | 0,45 | 0,25 |
| youtube_titles          | 2,484 | 3355,96 |   1,91 |  0,57 | 0,00 | 0,11 | 0,06 |
| **Average**             | 3,237 | 2787,81 | | | | | |

## Variant 4: VBE stored explicitly using u16, dictionary entries up to 16 bytes.

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| c_name      | 4,118 | 10566,18 | 1,72 | 0,38 | 0,03 | 0,01 |
| chinese     | 1,930 | 5283,37 | 0,70 | 0,33 | 0,02 | 0,02 |
| city        | 1,282 | 4230,67 | 0,12 | 0,08 | 0,00 | 0,00 |
| credentials | 1,527 | 5056,18 | 0,13 | 0,08 | 0,00 | 0,00 |
| email       | 1,917 | 4844,26 | 2,02 | 0,98 | 0,04 | 0,03 |
| faust       | 1,529 | 4690,19 | 0,29 | 0,17 | 0,01 | 0,01 |
| firstname   | 1,241 | 3772,05 | 0,36 | 0,28 | 0,01 | 0,01 |
| genome      | 2,029 | 2643,98 | 0,86 | 0,39 | 0,02 | 0,01 |
| hamlet      | 1,996 | 6581,30 | 0,26 | 0,12 | 0,01 | 0,01 |
| hex         | 1,121 | 3165,00 | 0,76 | 0,65 | 0,01 | 0,01 |
| japanese    | 1,953 | 6256,07 | 0,19 | 0,09 | 0,01 | 0,01 |
| l_comment   | 3,686 | 7647,65 | 2,50 | 0,62 | 0,04 | 0,02 |
| lastname    | 1,630 | 4076,20 | 2,15 | 1,24 | 0,04 | 0,04 |
| location    | 1,907 | 4875,34 | 2,52 | 1,25 | 0,04 | 0,03 |
| movies      | 1,573 | 3586,46 | 1,93 | 1,13 | 0,05 | 0,05 |
| ps_comment  | 4,616 | 10887,14 | 2,35 | 0,47 | 0,03 | 0,01 |
| street      | 1,594 | 5243,42 | 0,12 | 0,07 | 0,00 | 0,00 |
| urls        | 3,565 | 7492,15 | 5,94 | 1,55 | 0,07 | 0,05 |
| urls2       | 1,934 | 5092,28 | 1,57 | 0,76 | 0,03 | 0,02 |
| uuid        | 1,867 | 4460,02 | 3,43 | 1,75 | 0,05 | 0,04 |
| wiki        | 1,692 | 2441,32 | 2,14 | 1,15 | 0,06 | 0,05 |
| wikipedia   | 2,149 | 3211,73 | 2,71 | 1,13 | 0,08 | 0,05 |
| yago        | 1,570 | 3930,18 | 1,74 | 1,02 | 0,05 | 0,04 |
| **Average** | 2,106 |  5218,83 | | | | | 

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    | 2,998 | 3728,79 | 12,21 | 3,65 | 0,27 | 0,14 |
| amazon_beauty_reviews   | 3,176 | 3727,59 | 16,04 | 4,57 | 0,32 | 0,15 |
| amazon_handmade_titles  | 3,032 | 3228,28 | 16,05 | 4,68 | 0,41 | 0,20 |
| amazon_handmade_reviews | 3,256 | 3908,24 | 16,02 | 4,46 | 0,31 | 0,15 |
| ebay_titles             | 2,685 | 6590,50 | 1,69 | 0,55 | 0,05 | 0,03 |
| flipkart_titles         | 2,339 | 6732,56 | 0,77 | 0,28 | 0,03 | 0,02 |
| food_com_descriptions   | 3,295 | 3964,99 | 16,02 | 4,40 | 0,32 | 0,15 |
| food_com_reviews        | 3,278 | 3306,19 | 16,02 | 4,42 | 0,32 | 0,15 |
| food_com_titles         | 3,135 | 5955,82 | 6,17 | 1,76 | 0,14 | 0,07 |
| goodreads_descriptions  | 2,479 | 2767,66 | 16,05 | 5,88 | 0,37 | 0,21 |
| google_reviews          | 2,952 | 3502,17 | 16,00 | 4,91 | 0,34 | 0,17 |
| huffpost_descriptions   | 2,860 | 2952,50 | 16,04 | 5,06 | 0,37 | 0,19 |
| huffpost_headlines      | 2,579 | 2995,82 | 11,69 | 4,06 | 0,31 | 0,17 |
| imdb_reviews            | 2,920 | 3264,47 | 16,05 | 4,97 | 0,35 | 0,18 |
| linkedin_descriptions   | 3,304 | 3490,07 | 16,04 | 4,38 | 0,32 | 0,15 |
| linkedin_job_postings   | 3,574 | 4248,72 | 16,04 | 3,98 | 0,35 | 0,16 |
| myntra_titles           | 3,166 | 9109,74 | 0,63 | 0,17 | 0,02 | 0,01 |
| quora_questions         | 3,175 | 2023,72 | 16,05 | 4,55 | 0,34 | 0,17 |
| reddit_posts            | 2,876 | 3368,56 | 16,01 | 5,03 | 0,35 | 0,18 |
| reddit_titles           | 3,026 | 3599,09 | 16,05 | 4,79 | 0,34 | 0,17 |
| walmart_titles          | 1,852 | 4394,77 | 1,94 | 0,93 | 0,07 | 0,05 |
| wikipedia_movie_plots   | 2,917 | 3293,30 | 16,02 | 4,94 | 0,37 | 0,19 |
| youtube_comments        | 2,634 | 2984,68 | 16,01 | 5,47 | 0,39 | 0,21 |
| youtube_titles          | 2,394 | 5187,21 | 1,91 | 0,64 | 0,11 | 0,06 |
| **Average**             | 2,913 | 4096,89 | | | | | 

| Dataset | Compression Rate | Decompression Speed (MB/s) | Original Size (MB) | Data Size (MB) | Dictionary Data Size (MB) | Dictionary Separators Size (MB) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    | 2,998 | 3887,90 | 12,21 | 3,65 | 0,27 | 0,14 |
| amazon_beauty_reviews   | 3,633 | 3593,87 | 116,18 | 31,17 | 0,56 | 0,25 |
| amazon_handmade_titles  | 3,041 | 3352,98 | 16,37 | 4,77 | 0,41 | 0,21 |
| amazon_handmade_reviews | 3,727 | 3889,38 | 86,09 | 22,28 | 0,57 | 0,25 |
| ebay_titles             | 2,685 | 6516,39 | 1,69 | 0,55 | 0,05 | 0,03 |
| flipkart_titles         | 2,339 | 6528,04 | 0,77 | 0,28 | 0,03 | 0,02 |
| food_com_descriptions   | 3,631 | 3798,91 | 43,19 | 11,06 | 0,58 | 0,25 |
| food_com_reviews        | 3,822 | 2143,40 | 300,51 | 77,82 | 0,57 | 0,25 |
| food_com_titles         | 3,135 | 5900,51 | 6,17 | 1,76 | 0,14 | 0,07 |
| google_reviews          | 3,443 | 3694,93 | 95,94 | 27,09 | 0,53 | 0,25 |
| huffpost_descriptions   | 2,979 | 3195,39 | 22,89 | 6,94 | 0,50 | 0,25 |
| huffpost_headlines      | 2,579 | 3193,06 | 11,69 | 4,06 | 0,31 | 0,17 |
| imdb_reviews            | 3,243 | 3452,42 | 62,45 | 18,50 | 0,51 | 0,25 |
| linkedin_descriptions   | 3,371 | 3982,65 | 19,16 | 5,14 | 0,37 | 0,18 |
| linkedin_job_postings   | 4,134 | 2260,66 | 445,90 | 107,05 | 0,57 | 0,25 |
| myntra_titles           | 3,166 | 9436,15 | 0,63 | 0,17 | 0,02 | 0,01 |
| quora_questions         | 3,323 | 2333,07 | 22,97 | 6,23 | 0,46 | 0,22 |
| reddit_posts            | 3,049 | 3346,37 | 194,19 | 62,95 | 0,48 | 0,25 |
| reddit_titles           | 3,196 | 3561,81 | 32,04 | 9,28 | 0,50 | 0,25 |
| walmart_titles          | 1,852 | 4378,02 | 1,94 | 0,93 | 0,07 | 0,05 |
| wikipedia_movie_plots   | 3,168 | 3205,23 | 71,99 | 21,98 | 0,49 | 0,25 |
| youtube_comments        | 2,812 | 3111,27 | 56,23 | 19,30 | 0,45 | 0,25 |
| youtube_titles          | 2,394 | 5109,23 | 1,91 | 0,64 | 0,11 | 0,06 |
| **Average**             | 3,118 | 4081,37 | | | | | 
