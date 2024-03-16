# wallfuck
Wallpaper engine for Linux

My objective is to implement everything I can from scratch. Hopefully, I can learn a lot about AV programming that way :)

#What I've managed to implement
##Video
- Displays an image with the proper aspect ratio even if the window is resized
##Audio
###DSP
- Oscillators (Sine, Triangle, Square, Saw) with frequency and amplitude modulation
- White noise (uniform distribution from -1 to 1)
- Filters based on the first-order all-pass filter (all-pass, low-pass, high-pass) with cut-off modulation
- Filters based on the second-order all-pass filter (all-pass, stop-band, pass-band) with cut-off and curve modulation
- Multi-connection system for signal generator outputs
- Parallel: combine multiple generators into one by adding them together and dividing the output by the number of generators
- Effects chain
- Chain: way to combine a signal generator and an effects chain into one signal generator
- Moving average (needs to be reviewed as the performance is probably awful)
- Downsampler
- Mathematical operators
- Absolute value
###Misc
- Write to WAV file
- Fast fourier transform
- Inverse fast fourier transform
