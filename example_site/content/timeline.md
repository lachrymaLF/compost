% Timeline
% New!
% 2026-03-17 09:00 EDT

<c>
printf("<p>Hey there! We are in <span style=\"font-family: monospace;\">%s!</span></p>", THIS_FILE);

for (int i = 0; i < N_CONTENT_FILES; i++) {
    Date date = QueryFileDateTime(CONTENT_FILES[i]);
    printf("<a href=\"%s\">%s %d</a>\n", CONTENT_FILES[i] + 8, CONTENT_FILES[i], date.year);
}
</c>
