## Tokenizer Entropy

The tokenizer processes sequences of alphanumeric characters by grouping them into individual tokens. Non-alphanumeric characters are treated as separate tokens. After tokenization, the data is represented as a vector of tokens with length 'n' and entropy 'H'. The dictionary maps each token ID to its corresponding byte sequence, while the dictionary sizes refer to the lengths of these byte sequences. The average number of bytes per token is weighted by token frequencies. The compression rate is given by: dataset_size / (n*h + dict_values + dict_sizes).

| Dataset | Dataset Size (MB) | Entropy 'H' (b) | #Tokens 'n' | n*H (MB) | Dictionary Values (MB) | Dictionary Sizes (MB) | Avg Bytes per Token | Com Rate |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| amazon_beauty_titles | 12,211 | 6,777 | 4.373.600 | 3,533 | 0,408 | 0,254 | 2,928 | 2,91 |
| amazon_beauty_reviews | 116,184 | 6,094 | 50.188.466 | 36,462 | 0,987 | 0,480 | 2,427 | 3,06 |
| amazon_handmade_titles | 16,369 | 6,863 | 5.697.324 | 4,661 | 0,366 | 0,228 | 3,013 | 3,11 |
| amazon_handmade_reviews | 86,093 | 6,036 | 36.243.411 | 26,077 | 0,721 | 0,367 | 2,491 | 3,17 |
| ebay_titles | 1,686 | 6,372 | 635.908 | 0,483 | 0,126 | 0,087 | 2,781 | 2,42 |
| flipkart_titles | 0,772 | 6,700 | 263.895 | 0,211 | 0,076 | 0,052 | 3,067 | 2,28 |
| food_com_descriptions | 43,186 | 5,989 | 18.092.728 | 12,918 | 0,476 | 0,264 | 2,503 | 3,16 |
| food_com_reviews | 300,515 | 5,900 | 130.902.521 | 92,064 | 1,272 | 0,713 | 2,407 | 3,20 |
| food_com_titles | 6,168 | 6,466 | 1.866.990 | 1,439 | 0,186 | 0,110 | 3,464 | 3,56 |
| goodreads_descriptions | 1.566,236 | 7,351 | 601.992.298 | 527,522 | 27,898 | 12,047 | 2,728 | 2,76 |
| google_reviews | 95,940 | 6,337 | 39.400.599 | 29,766 | 0,982 | 0,564 | 2,553 | 3,06 |
| huffpost_descriptions | 22,886 | 6,776 | 8.936.362 | 7,218 | 0,652 | 0,356 | 2,685 | 2,78 |
| huffpost_headlines | 11,691 | 7,269 | 4.209.990 | 3,648 | 0,411 | 0,227 | 2,912 | 2,73 |
| imdb_reviews | 62,450 | 6,463 | 26.127.794 | 20,129 |0,908 | 0,494 | 2,506 | 2,90 |
| linkedin_descriptions | 19,162 | 6,671 | 6.624.613 | 5,269 | 0,575 | 0,315 | 3,033 | 3,11 |
| linkedin_job_postings | 445,902 | 6,723 | 146.609.695 | 117,502 | 4,796 | 1,688 | 3,189 | 3,60 |
| myntra_titles | 0,630 | 5,583 | 214.241 | 0,143 | 0,024 | 0,015 | 3,083 | 3,46 |
| quora_questions | 22,973 | 6,419 | 9.180.850 | 7,026 | 0,567 | 0,325 | 2,624 | 2,90 |
| reddit_posts | 194,191 | 6,775 | 81.618.406 | 65,921 | 3,304 | 1,436 | 2,495 | 2,75 |
| reddit_titles | 32,037 | 7,180 | 11.329.046 | 9,696 | 0,812 | 0,447 | 2,965 | 2,92 |
| walmart_titles | 1,941 | 7,408 | 702.971 | 0,621 |0,206 | 0,134 | 2,896 | 2,02 |
| wikipedia_movie_plots | 71,989 | 6,606 | 28.358.612 | 22,332 | 1,059 | 0,581 | 2,662 | 3,00 |
| youtube_comments | 56,234 | 7,063 | 23.819.355 | 20,056 | 1,370 | 0,734 | 2,476 | 2,54 |
| youtube_titles | 1,912 | 7,244 | 727.712 | 0,628 | 0,077 | 0,051 | 2,755 | 2,53 |