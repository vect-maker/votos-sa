```python
import os
import glob
print(os.listdir('.'))
for f in glob.glob('*'):
    print(f)


```

```text
['设备接入协议-TCP en-US.doc']
设备接入协议-TCP en-US.doc


```

```python
import docx

# Note: The file ends in .doc but starts with PK (zip format), so it is a docx.
doc = docx.Document('设备接入协议-TCP en-US.doc')
print("Paragraphs:", len(doc.paragraphs))
print("Tables:", len(doc.tables))

for p in doc.paragraphs[:15]:
    if p.text.strip():
        print(p.text)


```

```text
Paragraphs: 74
Tables: 8
Newland IoT Cloud Platform
 TCP Device Connection Protocol
 Description
	 The protocols described in this document are based on TCP and are defined entirely in JSON format.
 Onboarding Process
 Visit the platform at http://www.nlecloud.com to register an account
 The platform operates on a project basis. A single project can contain multiple devices, and each device can include multiple sensors. Therefore, add projects, devices, and sensors in that order. When adding a device, you can select TCP, MQTT, HTTP, or other communication protocols in the “Communication Protocol” field.
 When performing the above operations, be sure to record the following information: device ID (device) and device transmission key (key).
 The hardware device sends a TCP connection request to the following address and exchanges data packets with the platform using the following messages
 Platform access server address: newgateway.nlecloud.com or IP 120.77.58.34; TCP ports 8600, 8700, 8800
 Request Type


```

```python
# Let's inspect the entire document structure and dump it cleanly to markdown.
def dump_doc(doc):
    elements = []
    # Iterate through body elements in document order
    for block in doc.element.body:
        if block.tag.endswith('p'):
            text = block.text.strip()
            # Find style or text
            p_elem = [p for p in doc.paragraphs if p._element == block]
            if p_elem:
                p = p_elem[0]
                text = p.text.strip()
                if text:
                    elements.append(('p', p.style.name, text))
        elif block.tag.endswith('tbl'):
            tbl_elem = [t for t in doc.tables if t._element == block]
            if tbl_elem:
                t = tbl_elem[0]
                rows = []
                for row in t.rows:
                    cells = [cell.text.strip().replace('\n', ' ') for cell in row.cells]
                    rows.append(cells)
                elements.append(('tbl', rows))
    return elements

elements = dump_doc(doc)
for el in elements:
    if el[0] == 'p':
        print(f"[{el[1]}] {el[2]}")
    else:
        print("--- TABLE ---")
        for row in el[1]:
            print(" | ".join(row))
        print("-------------")


```

```text
[Normal] Newland IoT Cloud Platform
[Normal] TCP Device Connection Protocol
--- TABLE ---
Version | Date | Revision Details | Created/Revised by
v1.0 | July 19, 2017 | Initial Version | cs
v1.1 | June 13, 2018 | Revised the text in the "Heartbeat" section for smoother readability | cs
 |  |  | 

```

---

[Heading 1] Description
[Normal] The protocols described in this document are based on TCP and are defined entirely in JSON format.
[Heading 1] Onboarding Process
[List Paragraph] Visit the platform at http://www.nlecloud.com to register an account
[List Paragraph] The platform operates on a project basis. A single project can contain multiple devices, and each device can include multiple sensors. Therefore, add projects, devices, and sensors in that order. When adding a device, you can select TCP, MQTT, HTTP, or other communication protocols in the “Communication Protocol” field.
[List Paragraph] When performing the above operations, be sure to record the following information: device ID (device) and device transmission key (key).
[List Paragraph] The hardware device sends a TCP connection request to the following address and exchanges data packets with the platform using the following messages
[List Paragraph] Platform access server address: newgateway.nlecloud.com or IP 120.77.58.34; TCP ports 8600, 8700, 8800
[Heading 1] Request Type
--- TABLE ---
Type Value | Meaning | Direction
1 | CONN_REQ: Connection Request | C (client) → S (server)
2 | CONN_RESP: Connection Response | S → C
3 | PUSH_DATA: Data Upload | C→S
4 | PUSH _ACK: Data acknowledgment | S→C
5 | CMD_REQ: Command request | S->C
6 | CMD_RESP: Command Response | C→S
7 | PING_REQ: Heartbeat Request | S->C
8 | PING_RESP: Heartbeat Response | C<->S
Other values | Reserved |

---

[Heading 1] Connection Request (client)->(server)
[Normal] After a device establishes a TCP connection to the specified port, it must send a connection request message. The format of the request message is as follows:
[Normal] {
[Normal] "t": 1,
[Normal] "device": "P123456789",
[Normal] "key":"9861d43a0733415ab5424ee7d0f1c685",
[Normal] "ver": "v1.1"
[Normal] }
--- TABLE ---
JSON Key | JSON value | Description | Message Example
t | 1 | The integer 1, representing a connection request |
device | Device ID | Device ID when adding a device to the platform:  Newland Gateway: Go to Gateway Settings → [Parameter Settings] → [System Parameters] to find the serial number  Newland Agricultural Gateway: Log in to the agricultural gateway settings page via a browser → Device ID under [Device Status]  Newland Home Gateway: Go to the Home Gateway main interface on the tablet; the serial number is displayed in the top-left corner of the screen  Other devices (such as MCUs, SoCs, gateways, or mobile phones): You may enter a unique identifier of your choice to connect to the platform | PF12345678
Key | Transmission Key | A string of characters (32 characters long) automatically generated when adding a device to the platform; this value is globally unique; | 9861d43a0733415ab5424ee7d0f1c685
ver | Client Code Version | Can be a custom set of client code version numbers | V1.1

---

[Heading 1] Connection Response (server)->(client)
[Normal] After the hardware device client sends a connection request, the server sends a response message. The response message format is as follows:
[Normal] {
[Normal] "t": 2,
[Normal] "status": 0
[Normal] }
--- TABLE ---
JSON Key | JSON value | Description | Message Example
t | 2 | The integer 2, representing a connection response | 2
status | Status Result | One byte; depending on the validation result, the enumerated values are as follows  0: Handshake successful;  1: Handshake connection failed—protocol error;  2: Handshake failed—device not added;  3: Handshake failed—device authentication failed;  4: Handshake connection failed—not authorized;  5–255: Reserved values; | 0

---

[Normal] When the status is non-zero (failure): The server will not actively disconnect the device; it will wait 35 seconds before initiating a new connection request
[Heading 1] Data Upload (client) → (server)
[Normal] Once the device establishes a connection with the server, it can begin uploading sensor data. The data upload message format is as follows:
[Normal] {
[Normal] "t": 3,
[Normal] "datatype": 1,
[Normal] "datas": { See the table below for details } or [ See the table below for details ],
[Normal] "msgid": 123
[Normal] }
--- TABLE ---
JSON Key | JSON Value | Description | Message Example
t | 3 | The integer 3, indicating data upload | 3
datatype | Data reporting format type | Specifically, the sensor data format type within the `datas` attribute, as follows  = 1: JSON format (1 string);  = 2: JSON format, 2 strings;  = 3: JSON format 3 (string); | 1
datas | Array of sensor data to be reported | Depending on the `datatype`, this property can report data from multiple sensors or multiple data points from the same sensor. Here, `apitag1` is the sensor identifier, and `value` is the sensor value, which can be an integer, floating-point number, string, or binary (maximum size of 48 bytes).  Data type 1 (JSON format, 1 string):  "datas":  {   "apitag1": "value1",  "apitag2": value2,  …  } | Example:  "datas":  {   "temperature": 23.5,  "rgb-r": "#999",   …  }
datas | Array of sensor data to be reported | Data type is 2 (JSON format 2, string):  The data formats for `apitag1` and `value` are the same as above; `datetime1` must be in the yyyy-mm-dd hh:mm:ss format  "datas":  {   "apitag1":{"datetime1":"value1"},  "apitag2": {"datetime2":"value2"},  …  } | Example:  "datas":  {   "temperature": {"2015-03-22 22:31:12": 22.5},  …  }
datas | Array of sensor data to be reported | Data type is 3 (JSON format, 3 strings). Example:  The value data format is as above;  `dt` must be in the yyyy-mm-dd hh:mm:ss format  "datas":  [  {   "apitag":"temperature",  "datapoints":  [  {  "dt":"2018-01-22 22:22:22",   //optional "value": 36.5   // Floating-point number string  }  ]  },  {   "apitag": "location",  "datapoints":  […]  },  { … }  ] |
msgid | Message ID | A number generated by the client to identify this message; it is returned unchanged when the server sends an “Upload Response” | 123

---

[Heading 1] Data Upload Response (server) → (client)
[Normal] After the device reports sensor data, the server sends a confirmation message in the following format:
[Normal] {
[Normal] "t": 4,
[Normal] "msgid": 123,
[Normal] "status": 0
[Normal] }
--- TABLE ---
JSON key | JSON Value | Description | Message Example
t | 4 | The integer 4, representing a data upload response | 4
msgid | Message ID | The message ID of the client’s previous data report, returned by the server as-is | 123
status | Status Result | A single byte indicating  0: Report successful;  1: Upload failed;  Other: Reserved value; | 0

---

[Heading 1] Command Request (server) → (client)
[Normal] After a device successfully connects to the cloud platform, in addition to reporting sensor data, the server can also issue commands, such as controlling the on/off status of a sensor. The message format is as follows:
[Normal] {
[Normal] "t": 5,
[Normal] "cmdid": 123,
[Normal] "apitag": "rgb_open",
[Normal] "data":{ See the table below for details }
[Normal] }
--- TABLE ---
JSON Key | JSON Value | Description | Message Example
t | 5 | The integer 5 | 5
cmdid | Command ID | A message ID generated by the server; when the client device receives and processes the command and sends back a “Command Response,” it returns this ID unchanged to the server | 123
apitag | Sensor ID (optional) | The identifier used when adding a sensor to the platform | rgb_open
data | Command Value | A command value; can be an integer, floating-point number, string, or JSON | Example  Number: 1 Floating-point: 12.3  String: "Hello"  JSON: {"onoff": 1, "red": 23.5}

---

[Heading 1] Command Response (client) → (server)
[Normal] This message is sent by the client to the server in response to the corresponding command. The message format is as follows:
[Normal] {
[Normal] "t": 6,
[Normal] "cmdid": 123,
[Normal] "status": 0,
[Normal] "data": { See the table below for details }
[Normal] }
--- TABLE ---
JSON Key | JSON Value | Description | Message Example
t | 6 | The digit 6 | 6
cmdid | Command Number | The command number at the time of the request; returned as-is | 123
status | Status Result | A single byte indicating  0: Success;  1: Processing failed;  Other: Reserved value; | 0
data | Command response value | Response value; can be an integer, floating-point number, string, or JSON | Same as above

---

[Heading 1] Heartbeat Request (server) <-> (client)
[Normal] Heartbeat requests in the TCP protocol are bidirectional; that is, they can be sent proactively by the client to the server or by the server to the client, serving as a mechanism to notify the other party of their status.
[Normal] Client->Server: If there is no message exchange between the two parties (including this heartbeat message) within a fixed 50-second period, the server will proactively close the connection.
[Normal] Server->Client: If there is no message exchange between the two parties (including this heartbeat message) within a fixed 50-second period, the server will proactively send a heartbeat packet to confirm whether the client is still online; if no response packet is received from the client within 10 seconds after sending, the server will proactively close the connection.
[Normal] The fixed-string message is as follows:
[Normal] “$#AT#\r”
[Heading 1] Heartbeat Response (client) <-> (server)
[Normal] Regardless of the heartbeat request mode, the other party responds with the following string to notify the other of its status:
[Normal] “$OK##\r”

```

# Newland IoT Cloud Platform — TCP Device Connection Protocol[cite: 1]

```

---

## Revision History



| Version | Date | Revision Details | Created/Revised by |
| --- | --- | --- | --- |
| **v1.0** | July 19, 2017 | Initial Version | cs |
| **v1.1** | June 13, 2018 | Revised the text in the "Heartbeat" section for smoother readability | cs |

---

## 1. Description



The protocols described in this document are based on **TCP** and are defined entirely in **JSON** format.

---

## 2. Onboarding Process



* **Account Registration:** Register an account at [http://www.nlecloud.com](http://www.nlecloud.com).


* **Resource Hierarchy:** The platform operates on a project basis. A single project contains multiple devices, and each device can include multiple sensors. Add entities in the order: **Projects → Devices → Sensors**. Select `TCP` under the **Communication Protocol** field when adding a device.


* **Required Parameters:** Record the following credentials:


* **Device ID (`device`)**

* **Transmission Key (`key`)**



* **Server Connectivity:** Hardware establishes a TCP connection and exchanges packets with the following endpoints:


* **Domain:** `newgateway.nlecloud.com`

* **IP Address:** `120.77.58.34`

* **TCP Ports:** `8600`, `8700`, `8800`




---

## 3. Request Types



| Type Value (`t`) | Identifier / Meaning | Direction |
| --- | --- | --- |
| **1** | `CONN_REQ`: Connection Request | Client (C) → Server (S) |
| **2** | `CONN_RESP`: Connection Response | S → C |
| **3** | `PUSH_DATA`: Data Upload | C → S |
| **4** | `PUSH_ACK`: Data Acknowledgment | S → C |
| **5** | `CMD_REQ`: Command Request | S → C |
| **6** | `CMD_RESP`: Command Response | C → S |
| **7** | `PING_REQ`: Heartbeat Request | S → C |
| **8** | `PING_RESP`: Heartbeat Response | C ↔ S |
| **Other** | Reserved | — |

---

## 4. Connection Request (Client → Server)



Sent by the device immediately after establishing a TCP socket connection.

### Payload Format



```json
{
  "t": 1,
  "device": "P123456789",
  "key": "9861d43a0733415ab5424ee7d0f1c685",
  "ver": "v1.1"
}

```

### Parameter Reference



| Key | Type / Value | Description | Example |
| --- | --- | --- | --- |
| `t` | Integer (`1`) | Identifies the packet as a connection request.

 | `1` |
| `device` | String | **Device ID** configured in the platform:

 |  |



• *Newland Gateway:* **Gateway Settings → Parameter Settings → System Parameters** (Serial Number)



• *Newland Agricultural Gateway:* Web browser settings page → **Device Status → Device ID**



• *Newland Home Gateway:* Tablet UI top-left corner serial number



• *Other devices (MCU, SoC, mobile):* Custom unique identifier | `"PF12345678"` |
| `key` | String | **Transmission Key** (32-character globally unique token generated on device creation). | `"9861d43a0733415ab5424ee7d0f1c685"` |
| `ver` | String | Client code/firmware version identifier. | `"v1.1"` |

---

## 5. Connection Response (Server → Client)



Returned by the server following a connection request.

### Payload Format



```json
{
  "t": 2,
  "status": 0
}

```

### Parameter Reference



| Key | Type / Value | Description | Example |
| --- | --- | --- | --- |
| `t` | Integer (`2`) | Identifies the packet as a connection response.

 | `2` |
| `status` | Byte (Integer) | Authentication & status code:

 |  |



• `0`: Handshake successful



• `1`: Protocol error



• `2`: Device not added on platform



• `3`: Device authentication failed



• `4`: Device not authorized



• `5–255`: Reserved | `0` |

> **Failure Handling:** When `status != 0`, the server keeps the socket open for **35 seconds** before expecting/initiating a new connection attempt.
> 
> 

---

## 6. Data Upload (Client → Server)



Used by the device to push sensor measurements to the cloud platform.

### Payload Format



```json
{
  "t": 3,
  "datatype": 1,
  "datas": { ... },
  "msgid": 123
}

```

### Parameter Reference



| Key | Type / Value | Description | Example |
| --- | --- | --- | --- |
| `t` | Integer (`3`) | Identifies packet as data upload.

 | `3` |
| `datatype` | Integer | Format schema of the payload inside `datas`:

 |  |



• `1`: Simple Key-Value Object



• `2`: Timestamped Object



• `3`: Historical / Multi-datapoint Array | `1` |
| `datas` | Object / Array | Sensor telemetry payload (values can be integer, float, string, or binary up to 48 bytes; keys are platform `apitag` sensor IDs). | *(See details below)* |
| `msgid` | Integer | Client-generated sequence/message ID; echoed back in ACK. | `123` |

### `datas` Payload Schemas by `datatype`

#### Datatype 1: Key-Value Mapping



```json
{
  "t": 3,
  "datatype": 1,
  "datas": {
    "temperature": 23.5,
    "rgb-r": "#999"
  },
  "msgid": 123
}

```

#### Datatype 2: Timestamped Object



Timestamps must use `yyyy-MM-dd HH:mm:ss` format.

```json
{
  "t": 3,
  "datatype": 2,
  "datas": {
    "temperature": {
      "2015-03-22 22:31:12": 22.5
    }
  },
  "msgid": 124
}

```

#### Datatype 3: Multi-Point / Batch Historical Array



*`dt` format is `yyyy-MM-dd HH:mm:ss` (optional)*.

```json
{
  "t": 3,
  "datatype": 3,
  "datas": [
    {
      "apitag": "temperature",
      "datapoints": [
        {
          "dt": "2018-01-22 22:22:22",
          "value": 36.5
        }
      ]
    },
    {
      "apitag": "location",
      "datapoints": [
        {
          "value": "116.397128,39.916527"
        }
      ]
    }
  ],
  "msgid": 125
}

```

---

## 7. Data Upload Response (Server → Client)



Confirmation packet returned by the platform upon receiving sensor telemetry.

### Payload Format



```json
{
  "t": 4,
  "msgid": 123,
  "status": 0
}

```

### Parameter Reference



| Key | Type / Value | Description | Example |
| --- | --- | --- | --- |
| `t` | Integer (`4`) | Identifies packet as data upload acknowledgment.

 | `4` |
| `msgid` | Integer | Matches the client's original `msgid`.

 | `123` |
| `status` | Byte (Integer) | Processing result:

 |  |



• `0`: Success



• `1`: Failed



• Other: Reserved | `0` |

---

## 8. Command Request (Server → Client)



Issued by the platform to trigger actuators or request actions.

### Payload Format



```json
{
  "t": 5,
  "cmdid": 123,
  "apitag": "rgb_open",
  "data": 1
}

```

### Parameter Reference



| Key | Type / Value | Description | Example |
| --- | --- | --- | --- |
| `t` | Integer (`5`) | Identifies packet as a command request.

 | `5` |
| `cmdid` | Integer | Server-generated command identifier; must be echoed back in CMD response.

 | `123` |
| `apitag` | String (Optional) | Target sensor/actuator tag defined on the platform.

 | `"rgb_open"` |
| `data` | Any | Command parameter; can be Integer, Float, String, or JSON.

 | `1`, `12.3`, `"Hello"`, or `{"onoff": 1, "red": 23.5}`<br> |

---

## 9. Command Response (Client → Server)



Returned by the device after executing the command.

### Payload Format



```json
{
  "t": 6,
  "cmdid": 123,
  "status": 0,
  "data": 0
}

```

### Parameter Reference



| Key | Type / Value | Description | Example |
| --- | --- | --- | --- |
| `t` | Integer (`6`) | Identifies packet as a command response.

 | `6` |
| `cmdid` | Integer | Copied as-is from the corresponding `cmdid`.

 | `123` |
| `status` | Byte (Integer) | Command execution status:

 |  |



• `0`: Success



• `1`: Processing failed



• Other: Reserved | `0` |
| `data` | Any | Execution response payload (Integer, Float, String, or JSON). | `0` |

---

## 10. Heartbeat Mechanism



Heartbeat frames use fixed ASCII string delimiters rather than JSON objects.

### Behavior & Timeouts



* **Client → Server:** If no message exchange occurs within **50 seconds**, the server proactively terminates the TCP session.


* **Server → Client:** If no traffic is detected for **50 seconds**, the server sends a heartbeat inquiry. If the client fails to reply within **10 seconds**, the server closes the connection.



### Framing



| Operation | Direction | Exact Frame (ASCII / Escaped) |
| --- | --- | --- |
| **Heartbeat Request** | Client ↔ Server | `"$#AT#\r"`<br> |
| **Heartbeat Response** | Client ↔ Server | `"$OK##\r"`<br> |
