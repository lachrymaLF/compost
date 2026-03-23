#ifndef KPW_WEB_UTILS_H_
#define KPW_WEB_UTILS_H_

// Stupid macros for date/time stuff (so you could write 6+PM for instance)
#define AM 0
#define PM 12

// STB's image I/O library
#include "stb_image.h"
#include "stb_image_write.h"

// Basic types
typedef unsigned char byte;
typedef unsigned int uint;
#include <stdint.h>

/*
 * If a routine using anything from this library fails for some reason, the
 * user will see something like <Failed to render contents: "Bad malloc">
 */
static inline void ErrOut(const char *fmt, ...)
{
	va_list args;
	printf("<span class='comp-err'>&lt;Failed to render contents: \"");
	va_start(args, fmt);
	vprintf(fmt, args);
	va_end(args);
	printf("\"&gt;</span>");
	exit(1);
}

/*
 * Time/date utilities
 */
typedef struct Date
{
	int year;
	int month;
	int day;
	int hour;
	int minute;
	int second;
} Date;

static inline struct Date GetDateTime()
{
	time_t t = time(NULL);
	struct tm *tm = localtime(&t);
	
	return (struct Date) {
		tm->tm_year + 1900,
		tm->tm_mon + 1,
		tm->tm_mday,
		tm->tm_hour,
		tm->tm_min,
		tm->tm_sec
	};
}

// We include a little courtesy function for getting the current year, this
// is kinda nice for copyright info e.g. Copyright (C) 2020 kpworld.xyz
static inline void PrintCopyrightYear()
{
	printf("%d", GetDateTime().year);
}

// We also include a function for printing out the date an article was
// written. Takes in a date struct. Assumes CST.
// 
// Usage example: <c>PrintArticleTstamp((Date){2020, 2, 2, 8+PM, 16});</c>
//            ->  "Last updated February 2nd, 2020 at 8:16 PM CST"
//
// H O M E     S T Y L E
// 
static inline void PrintArticleTstamp(struct Date dt)
{
	char month_buf[10] = {0};
	switch (dt.month)
	{
	case 1:  strcat(month_buf, "January"); break;
	case 2:  strcat(month_buf, "February"); break;
	case 3:  strcat(month_buf, "March"); break;
	case 4:  strcat(month_buf, "April"); break;
	case 5:  strcat(month_buf, "May"); break;
	case 6:  strcat(month_buf, "June"); break;
	case 7:  strcat(month_buf, "July"); break;
	case 8:  strcat(month_buf, "August"); break;
	case 9:  strcat(month_buf, "September"); break;
	case 10: strcat(month_buf, "October"); break;
	case 11: strcat(month_buf, "November"); break;
	case 12: strcat(month_buf, "December"); break;
	default: ErrOut("Shit");
	}
	
	char day_buf[3] = {0};
	int ones = dt.day % 10;
	int tens = (int) floor(ones / 10) % 10;
	if (tens == 1)
	{
		strcat(day_buf, "th");
	}
	else
	{
		switch (ones)
		{
		case 1:  strcat(day_buf, "st"); break;
		case 2:  strcat(day_buf, "nd"); break;
		case 3:  strcat(day_buf, "rd"); break;
		default: strcat(day_buf, "th");
		}
	}
	
	int morning = dt.hour < PM;
	
	if (morning && dt.hour == 0)
		dt.hour = 12;
	
	printf("Last updated %s %d%s, %d", month_buf, dt.day, day_buf, dt.year);
	printf(" at %02d:%02d %s CST", dt.hour - (morning ? AM : PM), dt.minute, morning ? "AM" : "PM");
}

// Lastly, here's a function to query the last modified time of a certain file.
// You can use it like this: QueryFileDateTime(THIS_FILE)
static inline struct Date QueryFileDateTime(const char *path)
{
	struct stat attr;
	stat(path, &attr);
	struct tm *tm = localtime(&attr.st_mtime);
	
	return (struct Date) {
		tm->tm_year + 1900,
		tm->tm_mon + 1,
		tm->tm_mday,
		tm->tm_hour,
		tm->tm_min,
		tm->tm_sec
	};	
}

static inline time_t QueryFileTimestamp(const char *path)
{
	struct stat attr;
	stat(path, &attr);
	
	return attr.st_mtime;	
}

/*
 * Image I/O utilities
 */
typedef struct Bitmap
{
	int width;
	int height;
	int *data;
} Bitmap;

static inline struct Bitmap *CreateBitmap(int width, int height)
{
	struct Bitmap *bmp = malloc(sizeof(struct Bitmap));
	if (!bmp)
		ErrOut("Bad malloc (Bitmap)");
	
	bmp->width = width;
	bmp->height = height;
	
	bmp->data = malloc(sizeof(int) * bmp->width * bmp->height);
	if (!bmp->data)
		ErrOut("Bad malloc (Bitmap data)");
	
	return bmp;
}

static inline struct Bitmap *LoadBitmap(const char *path)
{
	int width = 0, height = 0;
	byte *data = stbi_load(path, &width, &height, NULL, 4);
	
	struct Bitmap *bmp = CreateBitmap(width, height);
	for (size_t i = 0; i < bmp->width * bmp->height; i++)
	{
		byte c[4];
		for (size_t j = 0; j < 4; j++)
			c[j] = data[i * 4 + j];
		
		bmp->data[i] = c[3] << 24 | c[0] << 16 | c[1] << 8 | c[2];
	}
	
	stbi_image_free(data);
	
	return bmp;
}

static inline void SaveBitmap(struct Bitmap *bmp, const char *path)
{
	int num_components = 4;
	byte *data = malloc(sizeof(byte) * bmp->width * bmp->height * num_components);
	for (size_t i = 0; i < bmp->width * bmp->height * num_components; i += num_components)
	{
		for (size_t j = 0; j < num_components; j++)
		{
			int offset = 24 - (j + 1) * 8;
			data[i + j] = (bmp->data[i / num_components] >> (offset >= 0 ? offset : 24)) & 0xFF;
		}	
	}
	stbi_write_png(path, bmp->width, bmp->height, 4, data, bmp->width * 4);
	free(data);
}

// Exactly the same as the above function, but inserts the image to the page
static inline void InsertBitmap(const char *path)
{
	printf("\n<img src=\"%s\" class=\"content-img\">\n", path);
}

static inline void DeleteBitmap(struct Bitmap *bmp)
{
	free(bmp);
	free(bmp->data);
}

#endif // KPW_WEB_UTILS_H_
