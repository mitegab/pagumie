/* Minimal C consumer of libethiopic.
 *
 * Build against the in-tree library:
 *
 *   cargo build -p ethiopic-ffi --release
 *   cc -I crates/ethiopic-ffi/include crates/ethiopic-ffi/examples/demo.c \
 *      -L target/release -lethiopic -o /tmp/demo
 *   LD_LIBRARY_PATH=target/release /tmp/demo
 *
 * Or, once the .deb is installed:  cc demo.c $(pkg-config --cflags --libs ethiopic)
 */
#include <ethiopic.h>

#include <stdio.h>
#include <string.h>
#include <time.h>

int main(void) {
    printf("libethiopic %s\n\n", eth_version());

    EthDate today;
    EthDateTime now;
    if (eth_now(ETH_EAT_UTC_OFFSET_SECONDS, ETH_DAY_START_DAWN, &now) != ETH_STATUS_OK) {
        fprintf(stderr, "eth_now failed\n");
        return 1;
    }
    today = now.date;

    char buf[256];
    eth_format(&today, "%A %-d %B %Y %N", ETH_LOCALE_AMHARIC, buf, sizeof buf);
    printf("today:        %s\n", buf);
    eth_format(&today, "%A %-d %B %Y %E", ETH_LOCALE_LATIN, buf, sizeof buf);
    printf("              %s\n", buf);
    printf("ethiopian time: %u:%02u %s\n", now.hour, now.minute,
           now.period == 0 ? "tewat" : now.period == 1 ? "keseat"
                          : now.period == 2 ? "mata" : "lelit");

    EthWesternDate g;
    eth_to_gregorian(&today, &g);
    printf("gregorian:    %04d-%02u-%02u\n", g.year, g.month, g.day);
    printf("jdn:          %lld\n", (long long)eth_to_jdn(&today));
    printf("leap year:    %s (Pagume has %u days)\n",
           eth_is_leap_year(today.year) ? "yes" : "no",
           eth_days_in_month(today.year, 13));

    EthComputus c;
    if (eth_computus(today.year, &c) == ETH_STATUS_OK) {
        EthWesternDate fg;
        eth_to_gregorian(&c.fasika, &fg);
        printf("\nBahire Hasab %d E.C. (A.A. %lld)\n", today.year, (long long)c.amete_alem);
        printf("  medeb %u  wenber %u  abektie %u  metqi %u\n",
               c.medeb, c.wenber, c.abektie, c.metqi);
        printf("  Fasika: %04d-%02u-%02u E.C. = %04d-%02u-%02u\n",
               c.fasika.year, c.fasika.month, c.fasika.day, fg.year, fg.month, fg.day);
        for (int i = 0; i < ETH_MOVABLE_FEAST_COUNT; i++) {
            EthDate d;
            if (eth_movable_feast(today.year, i, &d) != ETH_STATUS_OK) continue;
            printf("  %-22s %04d-%02u-%02u\n",
                   eth_movable_feast_name(i, ETH_LOCALE_LATIN), d.year, d.month, d.day);
        }
    }

    const char *holiday = eth_holiday_name(&today, ETH_LOCALE_LATIN);
    printf("\nholiday today: %s\n", holiday ? holiday : "(none)");
    return 0;
}
