# Pico Clock Green (Rust) Usage Guide

This guide does not reflect the offical Waveshare C implementation guide. There are different concepts and mappings to buttons. Whilst the configuration of this software will reach 100% support of the Waveshare implementation, it does not mean it is done the same way.

> This guide will be updated before the first official release of the software.

## App Switcher

The app switcher is a list of all apps that can be selected. The app name will show on the screen and you can use the buttons as outlined below to navigate the menu.

You can load the app switcher by performing a long press on the top button. This will happen no matter what else you are doing with the clock.

### Top Button

Select the currently shown app.

### Middle Button

View what the next app is (will cycle around when hitting the end).

### Bottom Button

View what the previous app was (will cycle around when hitting the start).

## Clock

The clock is the main app and will show the the current time as configured. It is currently responsible for showing the day of week and AM/PM time too.

### Top Button

Do nothing.

### Middle Button

Do nothing.

### Bottom Button

Do nothing.

## Pomodoro (Countdown)

The pomodoro is a timer that can currently countdown from X minutes, but no more than 60.

When the timer is running, no configuration changes can be made. However, before it is started, when it is paused or when it is completed, you are in "configuration" mode.

### Top Button (Timer Running)

This will pause the timer.

### Top Button (In Configuration)

This will start the timer.

### Middle Button (Timer Running)

This will do nothing.

### Middle Button (In Configuration)

#### Short press

This will increment the number of minutes by 1.

#### Long press

This reset the timer to 30 minutes.

### Bottom Button (Timer Running)

This will do nothing.

### Bottom Button (In Configuration)

#### Short press

This will decrement the number of minutes by 1.

#### Long press

This reset the timer to 30 minutes.

## Settings

The settings app is where all configuration for the clock is done. You will have to go through each setting to complete the journey, though you can exit early by going to the app switcher (just make sure you have completed and gone past the item you wanted to change)

### Top Button

Go to the next settings item and save the configuration.

> When modifying the time, this will set the seconds to 0. So make sure you modify you save at an appropriate time or the clock will become out of sync.

### Middle Button

This will increment the current active configuration. Will automatically wrap at maximum values (e.g. minute configuration will go from 59 -> 0).

### Bottom Button

This will decrement the current active configuration value. Will automatically wrap at minimum values (e.g. minute configuration will go from 0 -> 59).
