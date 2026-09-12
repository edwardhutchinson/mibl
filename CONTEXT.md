# SCOS MIB viewing

The terminology used to inspect spacecraft monitoring and commanding definitions in a SCOS-2000 Mission Information Base.

## Language

**MIB**:
The Mission Information Base containing the definitions used to interpret monitoring data and describe commands for a mission.

**Monitoring parameter**:
A named quantity described by the MIB, with characteristics such as its type and calibration. Its definition is distinct from where it occurs in a telemetry packet.

**Telemetry packet definition**:
The MIB description used to identify a kind of telemetry packet and describe its contents. It is distinct from an individual packet received from a spacecraft.

**Telecommand definition**:
The MIB description of a command and its arguments. It is distinct from a particular command invocation.

**Calibration definition**:
A description of how a parameter's raw representation relates to an engineering value or textual interpretation.

**Parameter occurrence**:
A monitoring parameter's declared place within a telemetry packet definition, including its location and any enclosing repetition or condition. One monitoring parameter can have multiple occurrences.

**Command argument**:
A parameter element within a telecommand definition, with its declared type, value rules, and position in any repetition structure. Its definition is distinct from the value supplied in a particular command invocation.
