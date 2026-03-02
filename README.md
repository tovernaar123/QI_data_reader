A python module for opening jpk-qi-data files, these files contain AFM qi data in the form of a zip that then contain all the pixel data. 
The module can be used like the following:

```py
from qi_data_reader import QiDataFIle
#opens the file and reads the main header of the file.
reader = QiDataFIle("data.jpk-qi-data")
#returns a list of strings which are all the channels that where stored in this data file.
channels = reader.get_channels()
#Every channel is measured using a daq and normally needs to be converted to some physical unit,
#this returns all units that the given channel can be converted too.
units = reader.get_channel_units("measuredHeight")
#Returns the data in the form of a numpy array, argument one is the channel, 2 is the segment (retract of extend) and the final is the name of unit (or conversion).
img = reader.get_channel_data(r'measuredHeight', 1, "nominal")

import numpy as np
import matplotlib.pyplot as plt
#For example the data can now be plotted.
plt.imshow(img, origin="lower", cmap="afmhot", interpolation="bicubic")
plt.colorbar(label="Height (m)")
plt.show()
```
