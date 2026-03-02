from qi_data_reader import QiDataFIle
import numpy as np
import time
import matplotlib.pyplot as plt
from matplotlib.colors import PowerNorm
reader = QiDataFIle("data.jpk-qi-data")
start = time.time()
channels = reader.get_channels()
print(channels)
units = reader.get_channel_units("measuredHeight")
print(f"got {units} for channel {channels[0]}")
data = reader.get_channel_data(r'measuredHeight', 1, "nominal")
data = np.min(data,axis=2)



img = np.abs(data)
import numpy as np
img = img - img.mean(axis=1, keepdims=True)

# norm = PowerNorm(gamma=0.25, vmin=0.0, vmax=10e-9)  # try 0.2–0.4

plt.imshow(img, origin="lower", cmap="afmhot", interpolation="bicubic")
plt.colorbar(label="|CAFM current| (A)")
end = time.time()
print(end-start)
plt.show()
print(data.shape)
