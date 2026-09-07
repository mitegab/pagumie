"""Independent Python reference implementation (Reingold & Dershowitz, CC 4th ed.).

Used only to generate cross-check vectors for the Rust crate's test suite.
Everything is expressed in Rata Die (R.D.): fixed day numbers, R.D. 1 = 1 Jan 1 CE (proleptic Gregorian).
"""
from math import floor

# JDN(noon) = RD + 1721425
RD_TO_JDN = 1721425

GREGORIAN_EPOCH = 1          # RD of 1 Jan 1 CE (Gregorian)
JULIAN_EPOCH = -1            # RD of 1 Jan 1 CE (Julian)
ETHIOPIC_EPOCH = 2796        # RD of 29 Aug 8 CE (Julian) = Meskerem 1, 1 A.M.
COPTIC_EPOCH = 103605        # RD of 29 Aug 284 CE (Julian) = Thoout 1, 1 A.M.

def gregorian_leap(y):
    return y % 4 == 0 and y % 400 not in (100, 200, 300)

def fixed_from_gregorian(y, m, d):
    return (GREGORIAN_EPOCH - 1 + 365 * (y - 1) + (y - 1) // 4 - (y - 1) // 100
            + (y - 1) // 400 + (367 * m - 362) // 12
            + (0 if m <= 2 else (-1 if gregorian_leap(y) else -2)) + d)

def gregorian_year_from_fixed(date):
    d0 = date - GREGORIAN_EPOCH
    n400, d1 = divmod(d0, 146097)
    n100, d2 = divmod(d1, 36524)
    n4, d3 = divmod(d2, 1461)
    n1 = d3 // 365
    year = 400 * n400 + 100 * n100 + 4 * n4 + n1
    return year if (n100 == 4 or n1 == 4) else year + 1

def gregorian_from_fixed(date):
    year = gregorian_year_from_fixed(date)
    prior_days = date - fixed_from_gregorian(year, 1, 1)
    correction = (0 if date < fixed_from_gregorian(year, 3, 1)
                  else (1 if gregorian_leap(year) else 2))
    month = (12 * (prior_days + correction) + 373) // 367
    day = date - fixed_from_gregorian(year, month, 1) + 1
    return (year, month, day)

def julian_leap(y):
    return y % 4 == (0 if y > 0 else 3)

def fixed_from_julian(y, m, d):
    y2 = y + 1 if y < 0 else y
    return (JULIAN_EPOCH - 1 + 365 * (y2 - 1) + (y2 - 1) // 4 + (367 * m - 362) // 12
            + (0 if m <= 2 else (-1 if julian_leap(y) else -2)) + d)

def julian_from_fixed(date):
    approx = (4 * (date - JULIAN_EPOCH) + 1464) // 1461
    year = approx - 1 if approx <= 0 else approx
    prior_days = date - fixed_from_julian(year, 1, 1)
    correction = (0 if date < fixed_from_julian(year, 3, 1)
                  else (1 if julian_leap(year) else 2))
    month = (12 * (prior_days + correction) + 373) // 367
    day = date - fixed_from_julian(year, month, 1) + 1
    return (year, month, day)

def fixed_from_ethiopic(y, m, d):
    return ETHIOPIC_EPOCH + 365 * (y - 1) + floor(y / 4) + 30 * (m - 1) + d - 1

def ethiopic_from_fixed(date):
    year = (4 * (date - ETHIOPIC_EPOCH) + 1463) // 1461
    month = (date - fixed_from_ethiopic(year, 1, 1)) // 30 + 1
    day = date + 1 - fixed_from_ethiopic(year, month, 1)
    return (year, month, day)

def ethiopic_leap(y):
    return y % 4 == 3

def coptic_from_fixed(date):
    year = (4 * (date - COPTIC_EPOCH) + 1463) // 1461
    month = (date - (COPTIC_EPOCH + 365 * (year - 1) + floor(year / 4))) // 30 + 1
    day = date - (COPTIC_EPOCH + 365 * (year - 1) + floor(year / 4) + 30 * (month - 1)) + 1
    return (year, month, day)

def orthodox_easter_rd(g_year):
    """Meeus Julian-calendar paschal computus (Astronomical Algorithms, ch. 8),
    returned as R.D. This is the independent cross-check for Bahire Hasab."""
    shifted_epact = (14 + 11 * (g_year % 19)) % 30
    j_year = g_year if g_year > 0 else g_year - 1
    paschal_moon = fixed_from_julian(j_year, 4, 19) - shifted_epact
    return kday_after(0, paschal_moon)  # the Sunday strictly after

def day_of_week_from_fixed(date):
    return date % 7  # 0 = Sunday

def kday_after(k, date):
    return date + 7 - ((date + 7 - k) % 7)


# ---------------------------------------------------------------- Baḥire Ḥasab
TEWSAK_OF_WEEKDAY = {0: 7, 1: 6, 2: 5, 3: 4, 4: 3, 5: 2, 6: 8}  # 0 = Sunday

MOVABLE_FEASTS = [
    ("nineveh", 0), ("abiy_tsome", 14), ("debre_zeit", 41), ("hosanna", 62),
    ("siklet", 67), ("tinsae", 69), ("rikbe_kahnat", 93), ("erget", 108),
    ("paraclete", 118), ("tsome_hawaryat", 119), ("tsome_dihnet", 121),
]

def bahire_hasab(y):
    aa = y + 5500
    medeb = aa % 19
    wenber = 18 if medeb == 0 else medeb - 1
    abektie = (wenber * 11) % 30
    metqi = (wenber * 19) % 30
    if metqi == 0:
        bm_month, bm_day = 1, 30
    elif metqi > 14:
        bm_month, bm_day = 1, metqi
    else:
        bm_month, bm_day = 2, metqi
    bm_rd = fixed_from_ethiopic(y, bm_month, bm_day)
    tewsak = TEWSAK_OF_WEEKDAY[bm_rd % 7]
    nineveh = bm_rd + 120 + tewsak
    return {
        "amete_alem": aa, "medeb": medeb, "wenber": wenber,
        "abektie": abektie, "metqi": metqi,
        "beale_metqi": (bm_month, bm_day), "mebaja_hamer": metqi + tewsak,
        "evangelist": aa % 4,               # 0 Yoḥannes 1 Matewos 2 Marqos 3 Luqas
        "new_year_weekday": fixed_from_ethiopic(y, 1, 1) % 7,
        "feasts": {n: nineveh + off for n, off in MOVABLE_FEASTS},
    }
