## Integer Compression Techniques

The results below show the execution of 5000 merges of the byte pair encoding algorithm after tokenizing the words (i.e., sequences of alphanumeric ASCII characters).

- `n` is the number of (non-unique) token IDs obtained at the end of the merges, representing the compressed strings.
- `H` is the entropy of the vector containing the n token IDs.
- **Dictionary Size** includes both the byte sequences of the entries and the space required for separators (4 bytes per separator).
- **Entropy C. Rate** is the compression rate, considering `n*H` space to represent the token IDs plus the dictionary space.
- The remaining compression rates use different integer compression techniques to encode the token IDs.

### Sample Datasets

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


### Complete Datasets

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

