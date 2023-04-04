# Timestamp

> This is a crate to convert the timestamp to DateTime.

## example

``` rust
let date_time = DateTime::new(1680623972); // timestamp
let year = date_time.year;
let month = date_time.month;
let day = date_time.day;
let hour = date_time.hour;
let minutes = date_time.minutes;
let seconds = date_time.seconds;

info!(
    "the standard Beijing time: {}   timestamp : {}",
    date_time, date_time.timestamp
);
```
