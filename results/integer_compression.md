## Integer Compression Techniques

The results below show the execution of `5000` merges of the byte pair encoding algorithm after tokenizing the words (i.e., sequences of alphanumeric ASCII characters).

- `n` is the number of (non-unique) token IDs obtained at the end of the merges, representing the compressed strings.
- `H` is the entropy of the vector containing the n token IDs.
- **Dictionary Size** includes both the byte sequences of the entries and the space required for separators (`4` bytes per separator).
- **Entropy C. Rate** is the compression rate, considering `n*H` space to represent the token IDs plus the dictionary space.
- The remaining compression rates use different integer compression techniques to encode the token IDs.

Here are the new results on sample datasets:

| Dataset | Original Size (MB) | n | H (b) | n*H (MB) | Dictionary Size (MB) | Entropy C. Rate | VBE C. Rate | Elias Gamma C. Rate | Elias Delta C. Rate | Fibonacci C. Rate |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    | 12,21 | 2.048.069 | 12,313 | 3,01 | 0,72 | 3,275 | 2,875 | 2,352 | 2,814 | 2,888 |
| amazon_beauty_reviews   | 16,04 | 2.786.973 | 12,125 | 4,03 | 0,52 | 3,523 | 3,067 | 2,463 | 2,954 | 3,068 |
| amazon_handmade_titles  | 16,05 | 2.719.977 | 12,138 | 3,94 | 0,65 | 3,499 | 3,058 | 2,482 | 2,999 | 3,073 |
| amazon_handmade_reviews | 16,02 | 2.678.162 | 12,097 | 3,86 | 0,50 | 3,670 | 3,190 | 2,565 | 3,075 | 3,194 |
| ebay_titles             |  1,69 |   195.488 | 12,178 | 0,28 | 0,30 | 2,904 | 2,661 | 2,337 | 2,639 | 2,676 |
| flipkart_titles         |  0,77 |    73.903 | 12,157 | 0,11 | 0,20 | 2,516 | 2,377 | 2,131 | 2,335 | 2,359 |
| food_com_descriptions   | 16,02 | 2.692.736 | 12,005 | 3,85 | 0,49 | 3,685 | 3,205 | 2,581 | 3,088 | 3,213 |
| food_com_reviews        | 16,02 | 2.638.470 | 12,082 | 3,80 | 0,46 | 3,755 | 3,255 | 2,610 | 3,128 | 3,255 |
| food_com_titles         |  6,17 |   856.824 | 12,092 | 1,24 | 0,37 | 3,853 | 3,388 | 2,796 | 3,290 | 3,405 |
| goodreads_descriptions  | 16,05 | 3.150.664 | 12,594 | 4,73 | 2,37 | 2,262 | 2,041 | 1,739 | 2,044 | 2,071 |
| google_reviews          | 16,00 | 2.800.671 | 12,199 | 4,07 | 0,62 | 3,411 | 2,992 | 2,409 | 2,893 | 2,991 |
| huffpost_descriptions   | 16,04 | 2.985.538 | 12,205 | 4,34 | 0,90 | 3,057 | 2,714 | 2,217 | 2,662 | 2,728 |
| huffpost_headlines      | 11,69 | 2.161.811 | 12,325 | 3,18 | 0,69 | 3,020 | 2,686 | 2,172 | 2,615 | 2,672 |
| imdb_reviews            | 16,05 | 2.861.524 | 12,347 | 4,21 | 0,82 | 3,187 | 2,819 | 2,290 | 2,747 | 2,826 |
| linkedin_descriptions   | 16,04 | 2.521.682 | 12,213 | 3,67 | 0,85 | 3,544 | 3,141 | 2,571 | 3,075 | 3,154 |
| linkedin_job_postings   | 16,04 | 2.396.185 | 12,086 | 3,45 | 0,82 | 3,752 | 3,333 | 2,720 | 3,249 | 3,330 |
| myntra_titles           |  0,63 |    42.348 | 11,842 | 0,06 | 0,14 | 3,104 | 2,912 | 2,674 | 2,871 | 2,920 |
| quora_questions         | 16,05 | 2.693.172 | 11,975 | 3,84 | 0,82 | 3,445 | 3,045 | 2,505 | 2,996 | 3,077 |
| reddit_posts            | 16,01 | 2.879.684 | 12,269 | 4,21 | 0,80 | 3,191 | 2,822 | 2,278 | 2,723 | 2,813 |
| reddit_titles           | 16,05 | 2.756.017 | 12,253 | 4,03 | 0,89 | 3,263 | 2,910 | 2,364 | 2,834 | 2,905 |
| walmart_titles          |  1,94 |   375.568 | 12,350 | 0,55 | 0,40 | 2,048 | 1,853 | 1,587 | 1,838 | 1,859 |
| wikipedia_movie_plots   | 16,02 | 2.834.708 | 12,348 | 4,17 | 0,75 | 3,256 | 2,898 | 2,332 | 2,820 | 2,884 |
| youtube_comments        | 16,01 | 3.300.228 | 12,431 | 4,89 | 1,03 | 2,705 | 2,391 | 1,947 | 2,336 | 2,399 |
| youtube_titles          |  1,91 |   324.766 | 11,770 | 0,46 | 0,17 | 3,057 | 2,737 | 2,247 | 2,656 | 2,708 |
| **Average**             |       |           |        |      |      | **3.208** | **2.849** | **2.349** | **2,778** | **2,853** |

Here are the new results on complete datasets:

| Dataset | Original Size (MB) | n | H (b) | n*H (MB) | Dictionary Size (MB) | Entropy C. Rate | VBE C. Rate | Elisa Gamma C. Rate | Elias Delta C. Rate | Fibonacci C. Rate
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    |  12,21 |  2.048.069 | 12,313 |  3,01 | 0,72 | 3,275 | 2,875 | 2,352 | 2,814 | 2,888 |
| amazon_beauty_reviews   | 116,18 | 20.200.231 | 12,152 | 29,26 | 1,52 | 3,774 | 3,256 | 2,584 | 3,131 | 3,259 |
| amazon_handmade_titles  |  16,37 |  2.776.078 | 12,139 |  4,02 | 0,66 | 3,502 | 3,060 | 2,484 | 3,002 | 3,076 |
| amazon_handmade_reviews |  86,09 | 14.289.797 | 12,114 | 20,64 | 1,14 | 3,953 | 3,407 | 2,708 | 3,278 | 3,413 |
| ebay_titles             |   1,69 |    195.488 | 12,178 |  0,28 | 0,30 | 2,904 | 2,661 | 2,337 | 2,639 | 2,676 |
| flipkart_titles         |   0,77 |     73.903 | 12,157 |  0,11 | 0,20 | 2,516 | 2,377 | 2,131 | 2,335 | 2,359 |
| food_com_descriptions   |  43,19 |  7.278.729 | 12,023 | 10,43 | 0,80 | 3,844 | 3,327 | 2,658 | 3,201 | 3,335 |
| food_com_reviews        | 300,51 | 49.645.211 | 12,113 | 71,69 | 2,05 | 4,076 | 3,497 | 2,760 | 3,349 | 3,496 |
| food_com_titles         |   6,17 |    856.824 | 12,092 |  1,24 | 0,37 | 3,853 | 3,388 | 2,796 | 3,290 | 3,405 |
| goodreads_descriptions  |        |            |        |       |      |       |       |       |       |       |
| google_reviews          |  95,94 | 17.214.797 | 12,223 | 25,08 | 1,57 | 3,599 | 3,129 | 2,486 | 3,021 | 3,130 |
| huffpost_descriptions   |  22,89 |  4.261.188 | 12,247 |  6,22 | 1,06 | 3,142 | 2,781 | 2,259 | 2,725 | 2,794 |
| huffpost_headlines      |  11,69 |  2.161.811 | 12,325 |  3,18 | 0,69 | 3,020 | 2,686 | 2,172 | 2,615 | 2,672 |
| imdb_reviews            |  62,45 | 11.151.713 | 12,376 | 16,45 | 1,46 | 3,487 | 3,054 | 2,442 | 2,970 | 3,062 |
| linkedin_descriptions   |  19,16 |  3.019.352 | 12,221 |  4,40 | 0,95 | 3,585 | 3,173 | 2,591 | 3,106 | 3,186 |
| linkedin_job_postings   | 445,90 | 66.928.902 | 12,166 | 97,07 | 6,54 | 4,304 | 3,772 | 3,000 | 3,664 | 3,770 |
| myntra_titles           |   0,63 |     42.348 | 11,842 |  0,06 | 0,14 | 3,104 | 2,912 | 2,674 | 2,871 | 2,920 |
| quora_questions         |  22,97 |  3.852.001 | 11,982 |  5,50 | 0,95 | 3,560 | 3,135 | 2,567 | 3,085 | 3,170 |
| reddit_posts            | 194,19 | 36.143.560 | 12,410 | 53,47 | 4,78 | 3,334 | 2,935 | 2,328 | 2,825 | 2,922 |
| reddit_titles           |  32,04 |  5.634.563 | 12,293 |  8,26 | 1,30 | 3,351 | 2,972 | 2,393 | 2,895 | 2,968 |
| walmart_titles          |   1,94 |    375.568 | 12,350 |  0,55 | 0,40 | 2,048 | 1,853 | 1,587 | 1,838 | 1,859 |
| wikipedia_movie_plots   |  71,99 | 12.741.479 | 12,458 | 18,92 | 1,69 | 3,492 | 3,083 | 2,455 | 3,007 | 3,079 |
| youtube_comments        |  56,23 | 11.680.652 | 12,490 | 17,39 | 2,12 | 2,882 | 2,529 | 2,035 | 2,469 | 2,540 |
| youtube_titles          |   1,91 |    324.766 | 11,770 |  0,46 | 0,17 | 3,057 | 2,737 | 2,247 | 2,656 | 2,708 |
| **Average**             |        |            |        |       |      | **3,377** | **2,983** | **2,437** | **2,904** | **2,986** |

### Limiting Token IDs to 2 Bytes

By analyzing the cumulative frequency of token IDs, we observe that the vast majority can be represented with just `2` bytes. Therefore, we can modify the VBE encoding to use up to `2` bytes, where a single flag bit distinguishes between token IDs that require only one byte and those that require both. 

To implement this strategy, we must abandon word-level tokenization, as it generates too many distinct token IDs. The new approach involves merging until the token ID reaches `2^15`. 

Another crucial factor is deciding whether to stop the merges prematurely; a poor choice here can significantly worsen the compression rate. Currently, we use a very simple stop condition: we halt as soon as we extract a pair with a frequency lower than `10`. This threshold was chosen empirically after testing several values and assessing the results. For now, this threshold provides a sufficient improvement in compression rate, but the timing of merge stops is essential and should be further optimized.

Here are the new results on sample datasets:

| Dataset | Original Size (MB) | n | H (b) | n*H (MB) | Dictionary Size (MB) | Entropy C. Rate | VBE C. Rate |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    | 12,21 | 1.913.412 | 13,546 | 3,09 | 0,39 | 3,51 | 3,21 |
| amazon_beauty_reviews   | 16,04 | 2.465.417 | 13,472 | 3,96 | 0,39 | 3,69 | 3,36 |
| amazon_handmade_titles  | 16,05 | 2.658.855 | 13,502 | 4,28 | 0,40 | 3,43 | 3,14 |
| amazon_handmade_reviews | 16,02 | 2.366.871 | 13,522 | 3,82 | 0,40 | 3,80 | 3,46 |
| ebay_titles             |  1,69 |   251.093 | 11,807 | 0,35 | 0,11 | 3,61 | 3,16 |
| flipkart_titles         |  0,77 |   129.061 | 11,217 | 0,17 | 0,06 | 3,28 | 2,82 |
| food_com_descriptions   | 16,02 | 2.328.441 | 13,495 | 3,75 | 0,41 | 3,86 | 3,53 |
| food_com_reviews        | 16,02 | 2.346.974 | 13,547 | 3,79 | 0,40 | 3,82 | 3,48 |
| food_com_titles         |  6,17 |   898.061 | 12,862 | 1,38 | 0,23 | 3,83 | 3,41 |
| goodreads_descriptions  | 16,05 | 3.395.723 | 13,447 | 5,44 | 0,33 | 2,78 | 2,53 |
| google_reviews          | 16,00 | 2.692.032 | 13,571 | 4,36 | 0,37 | 3,38 | 3,09 |
| huffpost_descriptions   | 16,04 | 2.845.865 | 13,495 | 4,58 | 0,36 | 3,25 | 2,98 |
| huffpost_headlines      | 11,69 | 2.256.852 | 13,525 | 3,64 | 0,35 | 2,93 | 2,69 |
| imdb_reviews            | 16,05 | 2.763.316 | 13,468 | 4,44 | 0,36 | 3,35 | 3,05 |
| linkedin_descriptions   | 16,04 | 2.342.033 | 13,659 | 3,81 | 0,40 | 3,81 | 3,50 |
| linkedin_job_postings   | 16,04 | 2.064.290 | 13,461 | 3,31 | 0,49 | 4,22 | 3,86 |
| myntra_titles           |  0,63 |    73.648 | 10,836 | 0,10 | 0,04 | 4,50 | 3,79 |
| quora_questions         | 16,05 | 2.403.341 | 13,512 | 3,87 | 0,40 | 3,76 | 3,45 |
| reddit_posts            | 16,01 | 2.796.307 | 13,477 | 4,49 | 0,37 | 3,29 | 3,00 |
| reddit_titles           | 16,05 | 2.606.631 | 13,446 | 4,18 | 0,37 | 3,53 | 3,24 |
| walmart_titles          |  1,94 |   486.963 | 12,350 | 0,72 | 0,12 | 2,33 | 2,03 |
| wikipedia_movie_plots   | 16,02 | 2.798.079 | 13,564 | 4,52 | 0,35 | 3,29 | 3,01 |
| youtube_comments        | 16,01 | 3.161.265 | 13,416 | 5,06 | 0,34 | 2,96 | 2,70 |
| youtube_titles          |  1,91 |   272.152 | 12,612 | 0,41 | 0,18 | 3,24 | 2,93 |
| **Average**             |       |           |        |      |      | **3,48** | **3,14** |

Here are the new results on complete datasets:

| Dataset | Original Size (MB) | n | H (b) | n*H (MB) | Dictionary Size (MB) | Entropy C. Rate | VBE C. Rate |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles    |  12,21 |  1.913.412 | 13,546 |  3,09 | 0,39 | 3,51 | 3,21 |
| amazon_beauty_reviews   | 116,18 | 17.956.720 | 13,464 | 28,82 | 0,39 | 3,98 | 3,60 |
| amazon_handmade_titles  |  16,37 |  2.713.511 | 13,501 |  4,37 | 0,40 | 3,44 | 3,15 |
| amazon_handmade_reviews |  86,09 | 12.676.600 | 13,518 | 20,43 | 0,40 | 4,13 | 3,74 |
| ebay_titles             |   1,69 |    251.093 | 11,807 |  0,35 | 0,11 | 3,61 | 3,16 |
| flipkart_titles         |   0,77 |    129.061 | 11,217 |  0,17 | 0,06 | 3,28 | 2,82 |
| food_com_descriptions   |  43,19 |  6.324.580 | 13,482 | 10,17 | 0,41 | 4,08 | 3,71 |
| food_com_reviews        | 300,51 | 44.444.977 | 13,532 | 71,70 | 0,40 | 4,17 | 3,77 |
| food_com_titles         |   6,17 |    898.061 | 12,862 |  1,38 | 0,23 | 3,83 | 3,41 |
| goodreads_descriptions  |        |            |        |       |      |      |      | 
| google_reviews          |  95,94 | 15.672.277 | 13,479 | 25,18 | 0,37 | 3,75 | 3,40 |
| huffpost_descriptions   |  22,89 |  4.086.197 | 13,479 |  6,57 | 0,36 | 3,31 | 3,02 |
| huffpost_headlines      |  11,69 |  2.256.852 | 13,525 |  3,64 | 0,35 | 2,93 | 2,69 |
| imdb_reviews            |  62,45 | 10.818.483 | 13,460 | 17,36 | 0,36 | 3,52 | 3,20 |
| linkedin_descriptions   |  19,16 |  2.805.762 | 13,654 |  4,57 | 0,40 | 3,86 | 3,54 |
| linkedin_job_postings   | 445,90 | 60.583.337 | 13,618 | 98,35 | 0,49 | 4,51 | 4,11 |
| myntra_titles           |   0,63 |     73.648 | 10,836 |  0,10 | 0,04 | 4,50 | 3,79 |
| quora_questions         |  22,97 |  3.446.158 | 13,513 |  5,55 | 0,40 | 3,86 | 3,53 |
| reddit_posts            | 194,19 | 36.508.357 | 13,457 | 58,57 | 0,36 | 3,30 | 2,99 |
| reddit_titles           |  32,04 |  5.354.410 | 13,462 |  8,59 | 0,37 | 3,57 | 3,28 |
| walmart_titles          |   1,94 |    486.963 | 12,350 |  0,72 | 0,12 | 2,33 | 2,03 |
| wikipedia_movie_plots   |  71,99 | 12.956.182 | 13,534 | 20,90 | 0,35 | 3,39 | 3,08 |
| youtube_comments        |  56,23 | 11.327.043 | 13,403 | 18,10 | 0,34 | 3,05 | 2,76 |
| youtube_titles          |   1,91 |    272.152 | 12,612 |  0,41 | 0,18 | 3,24 | 2,93 |
| **Average**             |        |            |        |       |      | **3,62** | **3,26** |
