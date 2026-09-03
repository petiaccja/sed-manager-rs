## Table of Contents

- [1 DISCLAIMERS, NOTICES, AND LICENSE TERMS](#1-disclaimers-notices-and-license-terms)
- [2 CHANGE HISTORY](#2-change-history)
- [3 SCOPE](#3-scope)
  - [3.1 Storage Workgroup Specifications Purpose](#31-storage-workgroup-specifications-purpose)
  - [3.2 Scope and Intended Audience](#32-scope-and-intended-audience)
  - [3.3 Key Words](#33-key-words)
  - [3.4 Conventions](#34-conventions)
    - [3.4.1 Informative Text](#341-informative-text)
    - [3.4.2 Precedence](#342-precedence)
    - [3.4.3 Lists](#343-lists)
    - [3.4.4 Table Legend](#344-table-legend)
    - [3.4.5 Fonts](#345-fonts)
  - [3.5 Document References](#35-document-references)
  - [3.6 Document Precedence](#36-document-precedence)
  - [3.7 Dependencies on Other Feature Sets](#37-dependencies-on-other-feature-sets)
  - [3.8 Interactions with Other Feature Sets](#38-interactions-with-other-feature-sets)
- [4 Block SID Authentication Overview](#4-block-sid-authentication-overview)
- [5 SSC Specific Functionality](#5-ssc-specific-functionality)
- [6 Feature Set Requirements](#6-feature-set-requirements)
  - [6.1 Level 0 Discovery](#61-level-0-discovery)
    - [6.1.1 Block SID Authentication Feature (Feature Code = 0402)](#611-block-sid-authentication-feature-feature-code-0402)
      - [6.1.1.1 Level 0 requirements for the Block SID Authentication Feature Descriptor](#6111-level-0-requirements-for-the-block-sid-authentication-feature-descriptor)
      - [6.1.1.2 SID Value State](#6112-sid-value-state)
      - [6.1.1.3 SID Authentication Blocked State](#6113-sid-authentication-blocked-state)
      - [6.1.1.4 Locking SP Freeze Lock supported](#6114-locking-sp-freeze-lock-supported)
      - [6.1.1.5 Locking SP Freeze Lock State](#6115-locking-sp-freeze-lock-state)
      - [6.1.1.6 Hardware Reset](#6116-hardware-reset)
  - [6.2 Block SID Authentication Command (M)](#62-block-sid-authentication-command-m)
    - [6.2.1 Command Structure and Execution](#621-command-structure-and-execution)
    - [6.2.2 Command Operation](#622-command-operation)
    - [6.2.3 Clear Events](#623-clear-events)
    - [6.2.4 Freeze SPs](#624-freeze-sps)
  - [6.3 Life Cycle](#63-life-cycle)
    - [6.3.1 Locking SP Manufactured-Frozen Life Cycle State (O)](#631-locking-sp-manufactured-frozen-life-cycle-state-o)
    - [6.3.2 Additional Life Cycle State Transitions](#632-additional-life-cycle-state-transitions)
      - [6.3.2.1 Manufactured to Manufactured-Frozen](#6321-manufactured-to-manufactured-frozen)
      - [6.3.2.2 Manufactured-Frozen to Manufactured](#6322-manufactured-frozen-to-manufactured)
      - [6.3.2.3 Manufactured-Frozen to Manufactured-Inactive](#6323-manufactured-frozen-to-manufactured-inactive)
  - [6.4 Locking SP](#64-locking-sp)
  - [6.5 Additional SPs](#65-additional-sps)

# 1 DISCLAIMERS, NOTICES, AND LICENSE TERMS

THIS SPECIFICATION IS PROVIDED “AS IS” WITH NO WARRANTIES WHATSOEVER, INCLUDING ANY WARRANTY OF MERCHANTABILITY, NONINFRINGEMENT, FITNESS FOR ANY PARTICULAR PURPOSE, OR ANY WARRANTY OTHERWISE ARISING OUT OF ANY PROPOSAL, SPECIFICATION OR SAMPLE.

Without limitation, TCG disclaims all liability, including liability for infringement of any proprietary rights, relating to use of information in this specification and to the implementation of this specification, and TCG disclaims all liability for cost of procurement of substitute goods or services, lost profits, loss of use, loss of data or any incidental, consequential, direct, indirect, or special damages, whether under contract, tort, warranty or otherwise, arising in any way out of use or reliance upon this specification or any information herein.

This document is copyrighted by Trusted Computing Group (TCG), and no license, express or implied, is granted herein other than as follows: You may not copy or reproduce the document or distribute it to others without written permission from TCG, except that you may freely do so for the purposes of (a) examining or implementing TCG specifications or (b) developing, testing, or promoting information technology standards and best practices, so long as you distribute the document with these disclaimers, notices, and license terms.

Contact the Trusted Computing Group at www.trustedcomputinggroup.org for information on specification licensing through membership agreements.

Any marks and brands contained herein are the property of their respective owners.

# 2 CHANGE HISTORY

| REVISION | DATE | DESCRIPTION |
| --- | --- | --- |
| 1.00/1.00 | August 5, 2015 | Adds new Block SID Authentication command which indicates to the device that all attempts to authenticate as SID should be blocked until a defined Clear Event has occurred  <br> Adds Block SID Authentication Feature Descriptor to Level 0 Discovery |
| 1.01/1.00 | February 12, 2021 | Adds optional Freeze Locking SP capability to Block SID Authentication command which indicates to the device that the Locking SP should enter a Frozen state until a defined Clear Event has occurred  <br> Adds Locking SP Freeze Lock Supported and Locking SP Freeze Lock State bits to Block SID Authentication Feature Descriptor which help identify support for and current state of the new Freeze Locking SP capability |

**CONTENTS**

[DISCLAIMERS, NOTICES, AND LICENSE TERMS [1](#disclaimers-notices-and-license-terms)](#disclaimers-notices-and-license-terms)

[CHANGE HISTORY [2](#change-history)](#change-history)

[1 SCOPE [4](#scope)](#scope)

[1.1 Storage Workgroup Specifications Purpose [4](#storage-workgroup-specifications-purpose)](#storage-workgroup-specifications-purpose)

[1.2 Scope and Intended Audience [4](#scope-and-intended-audience)](#scope-and-intended-audience)

[1.3 Key Words [4](#key-words)](#key-words)

[1.4 Conventions [4](#conventions)](#conventions)

[1.4.1 Informative Text [4](#informative-text)](#informative-text)

[1.4.2 Precedence [4](#precedence)](#precedence)

[1.4.3 Lists [5](#lists)](#lists)

[1.4.4 Table Legend [5](#table-legend)](#table-legend)

[1.4.5 Fonts [6](#fonts)](#fonts)

[1.5 Document References [6](#document-references)](#document-references)

[1.6 Document Precedence [6](#document-precedence)](#document-precedence)

[1.7 Dependencies on Other Feature Sets [7](#dependencies-on-other-feature-sets)](#dependencies-on-other-feature-sets)

[1.8 Interactions with Other Feature Sets [7](#interactions-with-other-feature-sets)](#interactions-with-other-feature-sets)

[2 Block SID Authentication Overview [8](#block-sid-authentication-overview)](#block-sid-authentication-overview)

[3 SSC Specific Functionality [9](#ssc-specific-functionality)](#ssc-specific-functionality)

[4 Feature Set Requirements [10](#feature-set-requirements)](#feature-set-requirements)

[4.1 Level 0 Discovery [10](#level-0-discovery)](#level-0-discovery)

[4.1.1 Block SID Authentication Feature (Feature Code = 0402) [10](#block-sid-authentication-feature-feature-code-0402)](#block-sid-authentication-feature-feature-code-0402)

[4.2 Block SID Authentication Command (M) [11](#block-sid-authentication-command-m)](#block-sid-authentication-command-m)

[4.2.1 Command Structure and Execution [11](#command-structure-and-execution)](#command-structure-and-execution)

[4.2.2 Command Operation [12](#command-operation)](#command-operation)

[4.2.3 Clear Events [13](#clear-events)](#clear-events)

[4.2.4 Freeze SPs [14](#freeze-sps)](#freeze-sps)

[4.3 Life Cycle [15](#life-cycle)](#life-cycle)

[4.3.1 Locking SP Manufactured-Frozen Life Cycle State (O) [15](#locking-sp-manufactured-frozen-life-cycle-state-o)](#locking-sp-manufactured-frozen-life-cycle-state-o)

[4.3.2 Additional Life Cycle State Transitions [16](#additional-life-cycle-state-transitions)](#additional-life-cycle-state-transitions)

[4.4 Locking SP [17](#locking-sp)](#locking-sp)

[4.5 Additional SPs [17](#additional-sps)](#additional-sps)

# 3 SCOPE

## 3.1 Storage Workgroup Specifications Purpose

The Storage Workgroup specifications provide a comprehensive architecture for putting Storage Devices under policy control as determined by the trusted platform host, the capabilities of the Storage Device to conform with the policies of the trusted platform, and the life cycle state of the Storage Device as a Trusted Peripheral.

## 3.2 Scope and Intended Audience

This specification defines the Block SID Authentication Feature. Any Storage Device that claims Block SID Authentication compatibility SHALL conform to this specification.

The intended audience for this specification is both trusted Storage Device manufacturers and developers that want to use these Storage Devices in their systems.

## 3.3 Key Words

Key words are used to signify SSC requirements.

The Key Words “**SHALL**”, “**SHALL NOT**”, “**SHOULD**,” and “**MAY**” are used in this document. These words are a subset of the RFC 2119 key words used by TCG, and have been chosen since they map to key words used in T10/T13 specifications. These key words are to be interpreted as described in [1].

In addition to the above key words, the following are also used in this document to describe the requirements of particular features, including tables, methods, and usages thereof.

- **Mandatory (M):** When a feature is Mandatory, the feature SHALL be implemented. A Compliance test SHALL validate that the feature is operational.
- **Optional (O):** When a feature is Optional, the feature MAY be implemented. If implemented, a Compliance test SHALL validate that the feature is operational.
- **Excluded (X):** When a feature is Excluded, the feature SHALL NOT be implemented. A Compliance test SHALL validate that the feature is not operational.
- **Not Required (N)** When a feature is Not Required, the feature MAY be implemented. No Compliance test is required.

## 3.4 Conventions

### 3.4.1 Informative Text

Informative text is used to provide background and context. Informative text does not define requirements. Informative text is formatted as follows:

*Begin Informative Comment* Hello World!

*End Informative Comment*

### 3.4.2 Precedence

The order of precedence to resolve conflicts between text, tables, or figures is text, then tables, then figures.

### 3.4.3 Lists

If the item in a list is not a complete sentence, the first word in the item is not capitalized. If the item in a list is a complete sentence, the first word in the item is capitalized.

Each item in a list ends with a semicolon, except the last item, which ends in a period. The next to the last entry in the list ends with a semicolon followed by an “and” or an “or” (i.e., “…; and”, or “…; or”). The “and” is used if all the items in the list are required. The “or” is used if only one or more items in the list are required.

Lists sequenced by letters show no ordering among the listed items. The leftmost level uses lower case letters and the next level uses capital letters. The following list shows no ordering among the named items: a) oak;

2. maple; and
3. soft wood:
  1. pine; or
  2. cedar.

List sequenced by numbers show an ordering relationship among the listed items. All levels use Arabic numerals. The following list shows an ordered relationship among the named items:

1) hydrogen; 2) helium; and 3) lithium:

1) lithium-6; and 2) lithium-7.

### 3.4.4 Table Legend

The following legend defines SP table cell coloring coding, with the RGB values for the shading of each cell indicated in parentheses. This color coding is informative only. The table cell content is normative.

**Table 1 SP Table Legend**

| Table Cell Legend | R-W | Value | Access Control |  | Comment |
| --- | --- | --- | --- | --- | --- |
| Arial-Narrow (230, 230, 230) | Read-only | Specified by specification | Fixed | •  <br> • | Cell content is Read-Only.  <br> Access control is fixed. |
|  |  |  |  | • | Value is specified by this specification. |
| Arial Narrow boldunder  <br> (230, 230, 230) | Read-only | VU | Fixed | •  <br> •  <br> • | Cell content is Read-Only.  <br> Access Control is fixed.  <br> Values are Vendor Unique (VU). A minimum or maximum value may be specified. |
| Table Cell Legend | R-W | Value | Access Control |  | Comment |
| Arial-Narrow  <br> (0, 0, 0) | Not Defined | (N) | Not Defined | •  <br> •  <br> • | Cell content is (N).  <br> Access control is not defined.  <br> Any text in table cell is informative only. |
|  |  |  |  | • | A Get MAY omit this column from the method response. |
| Arial Narrow boldunder  <br> (179, 179, 179) | Write | Preconfigured,  user  <br> personalizable | Preconfigured, user  <br> personalizable | •  <br> •  <br> • | Cell content is writable.  <br> Access control is personalizable  <br> Get Access Control is not described by this color coding |
| Arial-Narrow (179, 179, 179) | Write | Preconfigured, user  <br> personalizable | Fixed | •  <br> •  <br> • | Cell content is writable.  <br> Access control is fixed.  <br> Get Access Control is not described by this color coding |

### 3.4.5 Fonts

Names of methods and SP tables are in Courier New font (e.g., the Set method, the Locking table). This convention does not apply to method and table names appearing in headings or captions.

## 3.5 Document References

[1]. IETF RFC 2119, 1997, “Key words for use in RFCs to Indicate Requirement Levels”

[2]. Trusted Computing Group (TCG), “TCG Storage Architecture Core Specification”, Version 2.01

[3]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Opal”, Version 1.00

[4]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Opal”, Version 2.00

[5]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Opal”, Version 2.01

[6]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Opalite”, Version 1.00

[7]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Pyrite”, Version 1.00

[8]. Trusted Computing Group (TCG), “TCG Storage Interface Interactions Specification“, Version 1.09

[9]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Ruby”, Version 1.00

## 3.6 Document Precedence

In the event of conflicting information in this specification and other documents, the precedence for requirements is:

1. This specification and [3] or [4] or [5] or [6] or [7] or [9] (this document and an SSC are at the same level of precedence, and SHALL NOT conflict with each other)
2. TCG Storage Interface Interactions Specification [8]
3. TCG Storage Architecture Core Specification [2]

## 3.7 Dependencies on Other Feature Sets

This feature set has no dependencies on other feature sets.

## 3.8 Interactions with Other Feature Sets

This feature set has no interactions with other feature sets.

# 4 Block SID Authentication Overview

*Begin Informative Comment*

This specification defines a mechanism by which a host application can alert the storage device to block attempts to authenticate the SID authority until a subsequent device power cycle occurs.

This mechanism can be used by BIOS/platform firmware to prevent a malicious entity from taking ownership of a SID credential that is still set to its default value of MSID.

Additionally, this feature can optionally be used by BIOS/platform firmware to prevent a malicious entity with stolen credentials from making credential or access control changes that would lock out an authorized user. *End Informative Comment*

# 5 SSC Specific Functionality

This feature set requires no additional SSC-specific functionality.

# 6 Feature Set Requirements

This section defines the Mandatory (M) and Optional (O) requirements for the Block SID Authentication Feature Set.

## 6.1 Level 0 Discovery

A SD that implements the Block SID Authentication Feature Set SHALL return the Block SID Authentication Feature Descriptor as described in 4.1.1, in addition to the Level 0 Discovery response requirements defined in other applicable specifications.

### 6.1.1 Block SID Authentication Feature (Feature Code = 0402)

This feature descriptor SHALL be returned when the SD supports the Block SID Authentication Feature Set. The contents of the feature descriptor are defined in Table 2.

**Table 2 Level 0 Discovery - Block SID Authentication Feature Descriptor**

| Bit Byte | 7 | 6 |  | 5 | 4 |  | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) |  |  |  |  | Feature Code |  |  |  |  |
| 1 |  |  |  |  |  |  |  |  |  | (LSB) |
| 2 |  |  | Version |  |  |  |  | Reserved |  |  |
| 3 |  |  |  |  |  | Length |  |  |  |  |
| 4 |  |  | Reserved |  |  |  | Locking SP  <br> Freeze  <br> Lock  <br> State | Locking SP   <br> Freeze  <br> Lock supported | SID  <br> Authentic ation  <br> Blocked State | SID Value State |
| 5 |  |  |  |  |  | Reserved |  |  |  | Hardware Reset |
| 6-15 |  |  |  |  |  | Reserved |  |  |  |  |

#### 6.1.1.1 Level 0 requirements for the Block SID Authentication Feature Descriptor

Feature Code: SHALL be set to 0x0402

Version: SHALL be set to 0x2 or any version that supports the defined features in this specification Length: SHALL be set to 0x0C

#### 6.1.1.2 SID Value State

This field specifies whether the C_PIN_SID object’s PIN column value is equal to the C_PIN_MSID object’s PIN column value.

This bit SHALL be cleared to zero if the C_PIN_SID object’s PIN column value is equal to the C_PIN_MSID object’s PIN column value.

This bit SHALL be set to one if the C_PIN_SID object’s PIN column value is not equal to the C_PIN_MSID object’s PIN column value.

#### 6.1.1.3 SID Authentication Blocked State

This field specifies whether the authentication of the SID feature is blocked (see section 4.2.2).

This bit SHALL be cleared to zero if authentication of the SID authority is not blocked due to the Block SID Authentication command.

This bit SHALL be set to one if authentication of the SID authority is currently blocked due to the Block SID Authentication command.

#### 6.1.1.4 Locking SP Freeze Lock supported

This field specifies whether the Locking SP Freeze Lock capability is supported.

This bit SHALL be cleared to zero if the Locking SP Freeze Lock capability is not supported.

This bit SHALL be set to one if the Locking SP Freeze Lock capability is supported.

#### 6.1.1.5 Locking SP Freeze Lock State

This field specifies whether the Locking SP is in the Manufactured-Frozen life cycle state (see section 4.3.1).

This bit SHALL be cleared to zero if the Locking SP is not in the Manufactured-Frozen life cycle state.

This bit SHALL be set to one if the Locking SP is in the Manufactured-Frozen life cycle state.

#### 6.1.1.6 Hardware Reset

This bit SHALL be set to one if a Hardware Reset was selected in the Block SID Authentication command to be able to clear the SID Authentication Blocked State and the Locking SP Freeze Lock State bits.

This bit SHALL be cleared to zero if a Hardware Reset was not selected in the Block SID Authentication command to clear the SID Authentication Blocked State and the Locking SP Freeze Lock State bits.

*Begin Informative Comment*

The following events are always Clear Events (see 4.2.3), and as such there is no field in Level 0 discovery identifying that either has been selected as a Clear Event:

1. Power Cycle; and
2. Revert.

*End Informative Comment*

## 6.2 Block SID Authentication Command (M)

### 6.2.1 Command Structure and Execution

The Block SID Authentication command is delivered by the transport IF-SEND command.

If the Block SID Authentication command is supported, the TPer SHALL accept and acknowledge it at the interface level.

If the Block SID Authentication command is not supported, the TPer SHALL abort attempted invocations of the command at the interface level with the “Other Invalid Command Parameter” status (see [8]). There is no IF-RECV response to the Block SID Authentication command.

The Block SID Authentication command is defined in Table 3.

The Transfer Length SHALL be non-zero. Transferred data is formatted as indicated in Table 3.

The Clear Events field identifies the SD resets that clear the SID Authentication Blocked and Locking SP Freeze Lock states . See Table 4 for the structure of the Clear Events field.

**Table 3 Block SID Authentication Command**

| FIELD | VALUE |
| --- | --- |
| Command | IF-SEND |
| Protocol ID | 0x02 |
| Transfer Length | Non-zero |
| ComID | 0x0005 |
| Byte 0 | Clear Events (see Table 4) |
| Byte 1 | Freeze SPs (see Table 5) |
| Bytes 2 to Transfer Length – 1 | Reserved (00) |

### 6.2.2 Command Operation

If the SID C_PIN credential is not the same as the value of the MSID C_PIN credential, then the Block SID Authentication command SHALL result in success but SHALL have no effect on the SID Authentication Blocked State.

If the SID C_PIN credential is the same as the value of the MSID C_PIN credential, then upon successful completion of the Block SID Authentication command and until the next applicable SD Clear Event:

1. Otherwise valid invocations of the Authenticate method in which the Authority parameter is the SID authority’s UID SHALL result in a method status of SUCCESS, and a method result of False;
2. Otherwise valid invocations of the StartSession method in which the HostSigningAuthority parameter is the

SID authority’s UID SHALL result in a SyncSession method with a status of NOT AUTHORIZED; and

3. The Tries column of the SID C_PIN credential SHALL NOT be incremented as a result of authentication attempts that were unsuccessful due to the Block SID Authentication.

If:

3. the Locking SP Freeze Lock capability is supported;
4. the Locking SP is in the Manufactured life cycle state; and
5. a Freeze Locking SP bit (see section 4.2.4) in the Freeze SPs field of a Block SID Authentication command is set to one,

then the Locking SP SHALL transition to the Manufactured-Frozen life cycle state (see section 4.3.1) and the Locking SP Freeze Lock State SHALL be set to one upon successful completion of the Block SID Authentication command.

The Locking SP SHALL stay in the Manufactured-Frozen life cycle state until the next applicable SD Clear Event occurs.

If:

1. the Locking SP Freeze Lock capability is supported;
2. the Locking SP is in the Manufactured-Inactive life cycle state; and
3. the Freeze Locking SP bit (see section 4.2.4) in the Freeze SPs field of a Block SID Authentication command is set to one,

then the Freeze Locking SP bit SHALL be ignored.

If the Freeze SPs byte is not included in the payload for the Block SID Authentication command, then the TPer SHALL process the Block SID Authentication command as if the Freeze Locking SP bit was cleared to zero.

If:

1. the Locking SP Freeze Lock capability is not supported; and
2. the Freeze Locking SP bit (see section 4.2.4) in the Freeze SPs field of a Block SID Authentication command is set to one,

then the Block SID Authentication command SHALL fail with status “Other Invalid Command Parameter”.

If a Block SID Authentication command has been successfully executed and SID authentication is blocked or the Locking SP is in the Manufactured-Frozen life cycle state, then:

1. Subsequent invocations of the Block SID Authentication command SHALL fail with status “Other Invalid

Command Parameter”;

2. The SID Authentication Blocked State SHALL NOT change; and
3. Clear Events in effect SHALL remain the same as identified in the most recent successful invocation of the Block SID Authentication command.

After an applicable Clear Event occurs, attempts to authenticate the SID authority or start sessions with the Locking SP SHALL be processed normally until the Block SID Authentication command is successfully executed.

Clear Events selected by the successful completion of the Block SID Authentication command are reset when a Clear Event occurs.

### 6.2.3 Clear Events

Clear Events are mechanisms that reset the SID Authentication Blocked State and Locking SP Freeze Lock State bits, in order to permit normal authentication of the SID authority and use of the Locking SP. Clear Events also reset the current selection of host-selectable Clear Events.

The following SHALL always be Clear Events, and upon their occurrence SHALL clear the SID Authentication Blocked State and Locking SP Freeze Lock State bits and reset the selection of Clear Events:

1. A SD Power Cycle. See [8] for a mapping of TCG Storage Power Cycle reset type to resets defined by the underlying interface; and
2. A successful invocation of the Revert method on the Admin SP’s object in the Admin SP’s SP table. See [3], [4], [5], [6], [7], and [9] for SSC-specific definitions of the Revert method.

The following possible Clear Event MAY be selected by the host during execution of the Block SID Authentication:

a) Hardware Reset. See [8] for a mapping of TCG Storage Hardware Reset reset type to resets defined by the underlying interface.

A) A host selects Hardware Reset as a Clear Event by setting the Hardware Reset bit (Table 4) to one when invoking the Block SID Authentication command. After a successful completion of the Block SID Authentication command:

1. Any default Clear Events (e.g. Power Cycle, Revert) SHALL clear the SID Authentication Blocked State bit;
2. Any Clear Events supported by the device and selected in the command SHALL clear the SID Authentication Blocked State;
3. If the Locking SP is in the Manufactured-Frozen life cycle state, then any default Clear Events SHALL transition the Locking SP to the Manufactured life cycle and clear the Locking SP Freeze Lock State bit to zero;
4. If the Locking SP is in the Manufactured-Frozen life cycle state, then any Clear Events supported by the device and selected in the command SHALL transition the Locking SP to the Manufactured life cycle state and clear the Locking SP Freeze Lock State bit to zero; and
5. The Clear Events selected in the command SHALL NOT be modifiable by subsequent invocations of the Block SID Authentication command until after a Clear Event has occurred (see section 4.2.2).

**Table 4 Clear Events**

| Bit  <br> Byte | 7 | 6 | 5 | 4 |  | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB)  <br> |  |  |  | Reserved |  |  |  | Hardware  <br> Reset   <br> |

### 6.2.4 Freeze SPs

The Freeze SPs field allows the ability to specify specific SPs to be frozen as part of the Block SID Authentication command.

If the Locking SP Freeze Lock supported bit is set to one, then the TPer SHALL support freezing the Locking SP when the Freeze Locking SP bit is set to one.

*Begin Informative Comment*

This specification does not specify transitions from the Manufactured-Inactive life cycle state to Manufactured-Frozen life cycle state. The reason for this is that if the Locking SP was in the Manufactured-Inactive life cycle state where the Freeze Locking SP bit had been set to one, then the SID Authentication Blocked State bit would likely also be set to one, making it unlikely for an SP to ever transition from the Manufactured-Inactive life cycle state to the Manufactured-Frozen life cycle state.

*End Informative Comment*

If the Locking SP is in the Manufactured life cycle state and the TPer receives a Block SID Authentication command with the Freeze Locking SP bit set to one, then the Locking SP SHALL transition to the Manufactured-Frozen life cycle state. See section 4.3.1 for more details on the Manufactured-Frozen state and 4.3.2.1 for more details on the transition from Manufactured to Manufactured-Frozen life cycle state.

**Table 5 Freeze SPs**

| Bit Byte | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 |  |  |  | Reserved |  |  |  | Freeze  <br> Locking SP |

## 6.3 Life Cycle

For the Locking SP, this feature set defines an additional Optional Life Cycle State and additional Life Cycle State transitions.

### 6.3.1 Locking SP Manufactured-Frozen Life Cycle State (O)

The Manufactured-Frozen life cycle state occurs after the Locking SP has been Manufactured and the value of the

Locking SP’s Frozen column in the Admin SP’s SP table is True. See section 4.3.2 for details on how the Locking SP transitions between the Manufactured, Manufactured-Frozen, and Manufactured-Inactive life cycle states.

If the Locking SP is in the Manufactured-Frozen state, any attempt to start a session with the Locking SP SHALL result in a SyncSession with a status of SP_FROZEN.

If the Locking SP is in the Manufactured-Frozen life cycle state, then the Tries column of the C_PIN credential associated with any authority within the Locking SP SHALL NOT be incremented as a result of authentication attempts that were unsuccessful due to the Manufactured-Frozen life cycle state.

If an SD that supports the Block SID Authentication feature set also supports the Locking SP Freeze Lock capability, then the Locking SP SHALL support the Manufactured-Frozen life cycle state. See section 4.3.2 for details how the Locking SP transitions in and out of the Manufactured-Frozen life cycle state.

**Figure 1 Updated Life Cycle State Diagram for SSCs which support this feature set**

Note: Each SSC may specify different life cycle state requirements. This specification defines the ManufacturedFrozen life cycle state as an Optional life cycle state.

### 6.3.2 Additional Life Cycle State Transitions

This section identifies additional optional state transitions that are supported when the Locking SP Freeze Lock capability is supported (see section 4.1.1.4).

#### 6.3.2.1 Manufactured to Manufactured-Frozen

If the Locking SP Freeze Lock capability is supported, then the Locking SP SHALL transition from the Manufactured life cycle state to the Manufactured-Frozen life cycle state as a result of successful completion of the Block SID Authentication command with the Freeze Locking SP field set to one.

When the Locking SP transitions from the Manufactured life cycle state to the Manufactured-Frozen life cycle state:

1. The value of the Locking SP’s Frozen column in the Admin SP’s SP table SHALL be set to True.
2. The Locking SP Freeze Lock State bit SHALL be set to one.
3. The value of the Locking SP’s LifeCycle column in the Admin SP’s SP table SHALL be set to ManufacturedFrozen.

If the Locking SP transitions to the Manufactured-Frozen life cycle state, any open sessions with the Locking SP SHALL be aborted.

#### 6.3.2.2 Manufactured-Frozen to Manufactured

If the Locking SP is in the Manufactured-Frozen life cycle state, then the Locking SP SHALL transition from the Manufactured-Frozen life cycle state to the Manufactured life cycle state as a result of any default or selected Clear Event (see section 4.2.3) with the exception of successful invocation of the Revert method.

If the Original Factory State of the Locking SP is the Manufactured life cycle state and the Locking SP is in the

Manufactured-Frozen life cycle state, then successful invocation of the Revert method on the Admin SP or Locking SP SHALL transition the Locking SP from the Manufactured-Frozen life cycle state to the Manufactured life cycle state.

When the Locking SP transitions from the Manufactured-Frozen life cycle state to the Manufactured life cycle state:

1. The value of the Locking SP’s Frozen column in the Admin SP’s SP table SHALL be set to False.
2. The Locking SP Freeze Lock State bit SHALL be cleared to zero.
3. The value of the Locking SP’s LifeCycle column in the Admin SP’s SP table SHALL be set to Manufactured.

#### 6.3.2.3 Manufactured-Frozen to Manufactured-Inactive

If the Original Factory State of the Locking SP is the Manufactured-Inactive life cycle state and the Locking SP is in the Manufactured-Frozen life cycle state, then successful invocation of the Revert method on the Admin SP or the Locking SP SHALL transition the Locking SP from the Manufactured-Frozen life cycle state to the ManufacturedInactive life cycle state.

When the Locking SP transitions from the Manufactured-Frozen life cycle state to the Manufactured-Inactive life cycle state:

1. The value of the Locking SP’s Frozen column in the Admin SP’s SP table SHALL be set to False.
2. The Locking SP Freeze Lock State bit SHALL be cleared to zero.
3. The value of the Locking SP’s LifeCycle column in the Admin SP’s SP table SHALL be set to ManufacturedInactive.

## 6.4 Locking SP

This feature set requires no additions to the Locking SP.

## 6.5 Additional SPs

This feature set requires no additional SPs.
