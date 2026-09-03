**TCG Storage**

**Security Subsystem Class: Opalite**

**Specification Version 1.00**

**Revision 1.00**

**August 5, 2015**

Contact: admin@trustedcomputinggroup.org

**PUBLISHED**

Copyright © TCG 2015

Copyright © 2015 Trusted Computing Group, Incorporated.

**Disclaimers, Notices, and License Terms**

THIS SPECIFICATION IS PROVIDED “AS IS” WITH NO WARRANTIES WHATSOEVER, INCLUDING ANY WARRANTY OF MERCHANTABILITY, NONINFRINGEMENT, FITNESS FOR ANY PARTICULAR PURPOSE, OR ANY WARRANTY OTHERWISE ARISING OUT OF ANY PROPOSAL, SPECIFICATION OR SAMPLE.

Without limitation, TCG disclaims all liability, including liability for infringement of any proprietary rights, relating to use of information in this specification and to the implementation of this specification, and TCG disclaims all liability for cost of procurement of substitute goods or services, lost profits, loss of use, loss of data or any incidental, consequential, direct, indirect, or special damages, whether under contract, tort, warranty or otherwise, arising in any way out of use or reliance upon this specification or any information herein.

This document is copyrighted by Trusted Computing Group (TCG), and no license, express or implied, is granted herein other than as follows: You may not copy or reproduce the document or distribute it to others without written permission from TCG, except that you may freely do so for the purposes of (a) examining or implementing TCG specifications or (b) developing, testing, or promoting information technology standards and best practices, so long as you distribute the document with these disclaimers, notices, and license terms.

Contact the Trusted Computing Group at [www.trustedcomputinggroup.org](http://www.trustedcomputinggroup.org/) for information on specification licensing through membership agreements.

Any marks and brands contained herein are the property of their respective owners.

## Table of Contents

- [1 Introduction](#1-introduction)
  - [1.1 Document Purpose](#11-document-purpose)
  - [1.2 Scope and Intended Audience](#12-scope-and-intended-audience)
  - [1.3 Key Words](#13-key-words)
  - [1.4 Document References](#14-document-references)
  - [1.5 Document Precedence](#15-document-precedence)
  - [1.6 SSC Terminology](#16-ssc-terminology)
  - [1.7 Legend](#17-legend)
- [2 Opalite SSC Overview](#2-opalite-ssc-overview)
  - [2.1 Opalite SSC Use Cases and Threats](#21-opalite-ssc-use-cases-and-threats)
  - [2.2 Security Providers (SPs)](#22-security-providers-sps)
  - [2.3 Interface Communication Protocol](#23-interface-communication-protocol)
  - [2.4 Cryptographic Features](#24-cryptographic-features)
  - [2.5 Authentication](#25-authentication)
  - [2.6 Table Management](#26-table-management)
  - [2.7 Access Control & Personalization](#27-access-control-personalization)
  - [2.8 Issuance](#28-issuance)
  - [2.9 SSC Discovery](#29-ssc-discovery)
  - [2.10 Mandatory Feature Sets](#210-mandatory-feature-sets)
- [3 Opalite SSC Features](#3-opalite-ssc-features)
  - [3.1 Security Protocol 1 Support](#31-security-protocol-1-support)
    - [3.1.1 Level 0 Discovery (M)](#311-level-0-discovery-m)
      - [3.1.1.1 Level 0 Discovery Header](#3111-level-0-discovery-header)
      - [3.1.1.2 TPer Feature (Feature Code = 0x0001)](#3112-tper-feature-feature-code-0x0001)
      - [3.1.1.3 Locking Feature (Feature Code = 0x0002)](#3113-locking-feature-feature-code-0x0002)
        - [3.1.1.3.1 LockingEnabled Definition](#31131-lockingenabled-definition)
      - [3.1.1.4 Opalite SSC Feature (Feature Code = 0x0301)](#3114-opalite-ssc-feature-feature-code-0x0301)
  - [3.2 Security Protocol 2 Support](#32-security-protocol-2-support)
    - [3.2.1 ComID Management](#321-comid-management)
    - [3.2.2 Stack Protocol Reset (M)](#322-stack-protocol-reset-m)
    - [3.2.3 TPER_RESET command (M)](#323-tper_reset-command-m)
  - [3.3 Communications](#33-communications)
    - [3.3.1 Communication Properties](#331-communication-properties)
    - [3.3.2 Supported Security Protocols](#332-supported-security-protocols)
    - [3.3.3 ComIDs](#333-comids)
    - [3.3.4 Synchronous Protocol](#334-synchronous-protocol)
      - [3.3.4.1 Payload Encoding](#3341-payload-encoding)
        - [3.3.4.1.1 Stream Encoding Modifications](#33411-stream-encoding-modifications)
        - [3.3.4.1.2 TCG Packets](#33412-tcg-packets)
        - [3.3.4.1.3 Payload Error Response](#33413-payload-error-response)
    - [3.3.5 Storage Device Resets](#335-storage-device-resets)
      - [3.3.5.1 Interface Resets](#3351-interface-resets)
      - [3.3.5.2 TCG Reset Events](#3352-tcg-reset-events)
    - [3.3.6 Protocol Stack Reset Commands (M)](#336-protocol-stack-reset-commands-m)
- [4 Opalite SSC-compliant Functions and SPs](#4-opalite-ssc-compliant-functions-and-sps)
  - [4.1 Session Manager](#41-session-manager)
    - [4.1.1 Methods](#411-methods)
      - [4.1.1.1 Properties (M)](#4111-properties-m)
      - [4.1.1.2 StartSession (M)](#4112-startsession-m)
      - [4.1.1.3 SyncSession (M)](#4113-syncsession-m)
      - [4.1.1.4 CloseSession (O)](#4114-closesession-o)
  - [4.2 Admin SP](#42-admin-sp)
    - [4.2.1 Base Template Tables](#421-base-template-tables)
      - [4.2.1.1 SPInfo (M)](#4211-spinfo-m)
      - [4.2.1.2 SPTemplates (M)](#4212-sptemplates-m)
      - [4.2.1.3 Table (M)](#4213-table-m)
      - [4.2.1.4 MethodID (M)](#4214-methodid-m)
      - [4.2.1.5 AccessControl (M)](#4215-accesscontrol-m)
      - [4.2.1.6 ACE (M)](#4216-ace-m)
      - [4.2.1.7 Authority (M)](#4217-authority-m)
      - [4.2.1.8 C_PIN (M)](#4218-c_pin-m)
    - [4.2.2 Base Template Methods](#422-base-template-methods)
    - [4.2.3 Admin Template Tables](#423-admin-template-tables)
      - [4.2.3.1 TPerInfo (M)](#4231-tperinfo-m)
      - [4.2.3.2 Template (M)](#4232-template-m)
      - [4.2.3.3 SP (M)](#4233-sp-m)
    - [4.2.4 Admin Template Methods](#424-admin-template-methods)
    - [4.2.5 Crypto Template Tables](#425-crypto-template-tables)
    - [4.2.6 Crypto Template Methods](#426-crypto-template-methods)
      - [4.2.6.1 Random](#4261-random)
  - [4.3 Locking SP](#43-locking-sp)
    - [4.3.1 Base Template Tables](#431-base-template-tables)
      - [4.3.1.1 SPInfo (M)](#4311-spinfo-m)
      - [4.3.1.2 SPTemplates (M)](#4312-sptemplates-m)
      - [4.3.1.3 Table (M)](#4313-table-m)
      - [4.3.1.4 Type (N)](#4314-type-n)
      - [4.3.1.5 MethodID (M)](#4315-methodid-m)
      - [4.3.1.6 AccessControl (M)](#4316-accesscontrol-m)
      - [4.3.1.7 ACE (M)](#4317-ace-m)
      - [4.3.1.8 Authority (M)](#4318-authority-m)
      - [4.3.1.9 C_PIN (M)](#4319-c_pin-m)
      - [4.3.1.10 SecretProtect (M)](#43110-secretprotect-m)
    - [4.3.2 Base Template Methods](#432-base-template-methods)
    - [4.3.3 Crypto Template Tables](#433-crypto-template-tables)
    - [4.3.4 Crypto Template Methods](#434-crypto-template-methods)
      - [4.3.4.1 Random](#4341-random)
    - [4.3.5 Locking Template Tables](#435-locking-template-tables)
      - [4.3.5.1 LockingInfo (M)](#4351-lockinginfo-m)
      - [4.3.5.2 Locking (M)](#4352-locking-m)
        - [4.3.5.2.1 LockOnReset Restrictions](#43521-lockonreset-restrictions)
      - [4.3.5.3 MBRControl (M)](#4353-mbrcontrol-m)
        - [4.3.5.3.1 DoneOnReset Restrictions](#43531-doneonreset-restrictions)
      - [4.3.5.4 MBR (M)](#4354-mbr-m)
      - [4.3.5.5 K_AES_128 or K_AES_256 (M)](#4355-k_aes_128-or-k_aes_256-m)
    - [4.3.6 Locking Template Methods](#436-locking-template-methods)
    - [4.3.7 Non Template Tables](#437-non-template-tables)
      - [4.3.7.1 DataStore (M)](#4371-datastore-m)
- [5 Appendix – SSC Specific Features](#5-appendix-ssc-specific-features)
  - [5.1 Opalite SSC-Specific Methods](#51-opalite-ssc-specific-methods)
    - [5.1.1 Activate – Admin Template SP Object Method](#511-activate-admin-template-sp-object-method)
      - [5.1.1.1 Activate Support](#5111-activate-support)
      - [5.1.1.2 Side effects of Activate](#5112-side-effects-of-activate)
    - [5.1.2 Revert – Admin Template SP Object Method](#512-revert-admin-template-sp-object-method)
      - [5.1.2.1 Revert Support](#5121-revert-support)
      - [5.1.2.2 Side effects of Revert](#5122-side-effects-of-revert)
    - [5.1.3 RevertSP – Base Template SP Method](#513-revertsp-base-template-sp-method)
      - [5.1.3.1 RevertSP Support](#5131-revertsp-support)
      - [5.1.3.2 KeepGlobalRangeKey parameter (Locking Template-specific)](#5132-keepglobalrangekey-parameter-locking-template-specific)
      - [5.1.3.3 Side effects of RevertSP](#5133-side-effects-of-revertsp)
  - [5.2 Life Cycle](#52-life-cycle)
    - [5.2.1 Issued vs. Manufactured SPs](#521-issued-vs-manufactured-sps)
      - [5.2.1.1 Issued SPs](#5211-issued-sps)
      - [5.2.1.2 Manufactured SPs](#5212-manufactured-sps)
    - [5.2.2 Manufactured SP Life Cycle States](#522-manufactured-sp-life-cycle-states)
      - [5.2.2.1 State definitions for Manufactured SPs](#5221-state-definitions-for-manufactured-sps)
      - [5.2.2.2 State transitions for Manufactured SPs](#5222-state-transitions-for-manufactured-sps)
        - [5.2.2.2.1 Manufactured-Inactive to Manufactured](#52221-manufactured-inactive-to-manufactured)
        - [5.2.2.2.2 ANY STATE to ORIGINAL FACTORY STATE](#52222-any-state-to-original-factory-state)
      - [5.2.2.3 State behaviors for Manufactured SPs](#5223-state-behaviors-for-manufactured-sps)
        - [5.2.2.3.1 Manufactured-Inactive](#52231-manufactured-inactive)
        - [5.2.2.3.2 Manufactured](#52232-manufactured)
    - [5.2.3 Type Table Modification](#523-type-table-modification)
  - [5.3 Byte Table Access Granularity](#53-byte-table-access-granularity)
    - [5.3.1 Table Table Modification](#531-table-table-modification)
      - [5.3.1.1 MandatoryWriteGranularity](#5311-mandatorywritegranularity)
        - [5.3.1.1.1 Object Tables](#53111-object-tables)
        - [5.3.1.1.2 Byte Tables](#53112-byte-tables)
      - [5.3.1.2 RecommendedAccessGranularity](#5312-recommendedaccessgranularity)
        - [5.3.1.2.1 Object Tables](#53121-object-tables)
        - [5.3.1.2.2 Byte Tables](#53122-byte-tables)

# 1 Introduction

## 1.1 Document Purpose

The Storage Workgroup specifications provide a comprehensive architecture for putting Storage Devices under policy control as determined by the trusted platform host, the capabilities of the Storage Device to conform to the policies of the trusted platform, and the lifecycle state of the Storage Device as a Trusted Peripheral.

## 1.2 Scope and Intended Audience

This specification defines the Opalite Security Subsystem Class (SSC). Any SD that claims Opalite SSC compatibility SHALL conform to this specification.

The intended audience for this specification is both trusted Storage Device manufacturers and developers that want to use these Storage Devices in their systems.

## 1.3 Key Words

Key words are used to signify SSC requirements.

The Key Words “**SHALL**”, “**SHALL NOT**”, “**SHOULD**,” and “**MAY**” are used in this document. These words are a subset of the RFC 2119 key words used by TCG, and have been chosen since they map to key words used in T10/T13 specifications. These key words are to be interpreted as described in [1].

In addition to the above key words, the following are also used in this document to describe the requirements of particular features, including tables, methods, and usages thereof.

- **Mandatory (M):** When a feature is Mandatory, the feature SHALL be implemented. A Compliance test SHALL validate that the feature is operational.
- **Optional (O):** When a feature is Optional, the feature MAY be implemented. If implemented, a Compliance test SHALL validate that the feature is operational.
- **Excluded (X):** When a feature is Excluded, the feature SHALL NOT be implemented. A Compliance test SHALL validate that the feature is not operational.
- **Not Required (N)** When a feature is Not Required, the feature MAY be implemented. No Compliance test is required.

## 1.4 Document References

[1]. IETF RFC 2119, 1997, “Key words for use in RFCs to Indicate Requirement Levels”

[2]. Trusted Computing Group (TCG), “TCG Storage Architecture Core Specification”, Version 2.01

[3]. NIST, FIPS-197, 2001, “Advanced Encryption Standard (AES)”

[4]. Trusted Computing Group (TCG), “TCG Storage Interface Interactions Specification“, Version 1.04

[5]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Opal”, Version 1.00

[6]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Opal”, Version 2.01

[7]. Trusted Computing Group (TCG), “TCG Storage Opal SSC Feature Set: PSID”, Version 1.00

[8]. Trusted Computing Group (TCG), “TCG Storage Feature Set: Block SID Authentication”, Version 1.00

## 1.5 Document Precedence

In the event of conflicting information in this specification and other documents, the precedence for requirements is:

1. This specification
2. Storage Interface Interactions Specification [4]
3. TCG Storage Architecture Core Specification [2]

## 1.6 SSC Terminology

This section provides special definitions that are not defined in [2].

**Table 1 Opalite SSC Terminology**

| Term | Definition |
| --- | --- |
| Manufactured SP | A Manufactured SP is an SP that was created and preconfigured during the SD manufacturing process |
| N/A | Not Applicable. |
| Original Factory State (OFS) | The original state of an SP when it was created in manufacturing, including its table data, access control settings, and life cycle state.  Each Manufactured SP has its own Original Factory State.  <br> Original Factory State applies to Manufactured SPs only. |
| Vendor Unique (VU) | These values are unique to each SD manufacturer. Typically VU is used in table cells. |

## 1.7 Legend

The following legend defines SP table cell coloring coding. This color coding is informative only. The table cell content is normative.

**Table 2 SP Table Legend**

| Table  <br> Cell  <br> Legend | R-W | Value | Access Control |  | Comment |
| --- | --- | --- | --- | --- | --- |
| Arial-Narrow | Read-only | Opalite SSC specified | Fixed | •  <br> •  <br> • | Cell content is Read-Only.  <br> Access control is fixed.  <br> Value is specified by the Opalite  <br> SSC |
| Arial Narrow bold-under | Read-only | VU | Fixed | •  <br> •  <br> • | Cell content is Read-Only.  <br> Access Control is fixed.  <br> Values are Vendor Unique (VU). A minimum or maximum value may be specified. |
| Arial-Narrow | Not Defined | (N) | Not Defined | •  <br> •  <br> • | Cell content is (N).  <br> Access control is not defined. Any text in table cell is informative only. |
|  |  |  |  | • | A Get MAY omit this column from the method response. |
| Arial Narrow bold-under | Write | Preconfigured,  user personalizable | Preconfigured, user personalizable | •  <br> •  <br> • | Cell content is writable.  <br> Access control is personalizable Get Access Control is not described by this color coding |
| Arial-Narrow | Write | Preconfigured, user personalizable | Fixed | •  <br> •  <br> • | Cell content is writable.  <br> Access control is fixed. Get Access Control is not described by this color coding |

# 2 Opalite SSC Overview

## 2.1 Opalite SSC Use Cases and Threats

| Begin Informative Content  <br> The Opalite SSC is an implementation profile for Storage Devices built to: |  |
| --- | --- |
|  | Protect the confidentiality of stored user data against unauthorized access once it leaves the owner’s control (involving a power cycle and subsequent deauthentication)  <br> Enable interoperability between multiple SD vendors |
| An Opalite SSC compliant SD: |  |
|  | Facilitates feature discoverability  <br> Provides some user definable features (e.g. access control, user passwords, etc.)  <br> Supports Opalite SSC unique behaviors (e.g. communication, table management) |
| <br> This specification addresses a limited set of use cases. They are:  <br> Deploy Storage Device & Take Ownership: the Storage Device is integrated into its target system and ownership transferred by setting or changing the Storage Device’s owner credential.  <br> Activate or Enroll Storage Device: data encryption and access control credentials (re)generated and/or set on the Storage Device. Access control is configured for LBA range unlocking.  <br> Lock & Unlock Storage Device: unlocking and locking under host control via either an explicit lock or implicit lock triggered by a reset event. MBR shadowing provides a mechanism to boot into a secure preboot authentication environment to handle device unlocking.  <br> Repurpose & End-of-Life: erasure of data and reset of locking credential(s) for Storage Device repurposing or decommissioning.   <br> This specification is defined as a limited subset of [6], with the intent to provide a smaller set of features and use cases while remaining command compatible, such that an application that can manage an Opalite SSC-based SD can also manage a SD that is compatible with [6].  <br>   <br> End Informative Content |  |

## 2.2 Security Providers (SPs)

An Opalite SSC compliant SD SHALL support at least two Security Providers (SPs):

1. Admin SP
2. Locking SP

The Locking SP MAY be created by the SD manufacturer.

## 2.3 Interface Communication Protocol

An Opalite SSC compliant SD SHALL implement the synchronous communications protocol as defined in Section 3.3.4.

This communication protocol operates based upon configuration information defined by:

1. The values reported via Level 0 Discovery (Section 3.1.1)
2. The combination of the host's communication properties and the TPer's communication properties (see Properties Method Section 4.1.1.1)

## 2.4 Cryptographic Features

An Opalite SSC compliant SD SHALL implement Full Disk Encryption for all host accessible user data stored on media. AES-128 or AES-256 SHALL be supported (see [3]).

## 2.5 Authentication

An Opalite SSC compliant SD SHALL support password authorities and authentication.

## 2.6 Table Management

This specification defines the mandatory tables and mandatory/optional table rows delivered by the SD manufacturer. The creation or deletion of tables after manufacturing is outside the scope of this specification. The creation or deletion of table rows post-manufacturing is outside the scope of this specification.

## 2.7 Access Control & Personalization

Initial access control policies are preconfigured at SD manufacturing time on manufacturer created SPs. An Opalite SSC compliant SD SHALL support personalization of certain Access Control Elements of the Locking SP.

## 2.8 Issuance

The Locking SP MAY be present in the SD when the SD leaves the manufacturer. The issuance of SPs is outside the scope of this specification.

## 2.9 SSC Discovery

Refer to [2] for details (see section 3.1.1).

## 2.10 Mandatory Feature Sets

An Opalite SSC compliant SD SHALL support the following TCG Storage Feature Sets:

1. PSID, Opal SSC Feature Set (refer to [7]).
2. Block SID Authentication (refer to [8])

# 3 Opalite SSC Features

## 3.1 Security Protocol 1 Support

### 3.1.1 Level 0 Discovery (M)

Refer to [2] for more details.

An Opalite SSC compliant SD SHALL return the following Level 0 response:

- Level 0 Discovery Header
- TPer Feature Descriptor
- Locking Feature Descriptor
- Opalite SSC Feature Descriptor

#### 3.1.1.1 Level 0 Discovery Header

**Table 3 Level 0 Discovery Header**

| Bit Byte | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) |  |  | Length of Parameter Dat |  | a |  |  |
| 1 |  |  |  |  |  |  |  |  |
| 2 |  |  |  |  |  |  |  |  |
| 3 |  |  |  |  |  |  |  | (LSB) |
| 4 | (MSB) |  |  | Data structure revision |  |  |  |  |
| 5 |  |  |  |  |  |  |  |  |
| 6 |  |  |  |  |  |  |  |  |
| 7 |  |  |  |  |  |  |  | (LSB) |
| 8 | (MSB) |  |  | Reserved |  |  |  |  |
| … |  |  |  |  |  |  |  |  |
| 15 |  |  |  |  |  |  |  | (LSB) |
| 16 | (MSB) |  |  | Vendor Specific |  |  |  |  |
| … |  |  |  |  |  |  |  |  |
| 47 |  |  |  |  |  |  |  | (LSB) |

- Length of parameter data = VU
- Data structure revision = 0x00000001 or

any version that supports the defined features in this SSC

- Vendor Specific = VU

#### 3.1.1.2 TPer Feature (Feature Code = 0x0001)

**Table 4 Level 0 Discovery - TPer Feature Descriptor**

| Bit Byte | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) |  |  | Feature Code (0x0001) |  |  |  |  |
| 1 |  |  |  |  |  |  |  | (LSB) |
| 2 |  | Version |  |  |  | Reserved |  |  |
| 3 |  |  |  | Length |  |  |  |  |
| 4 | Reserved | ComID Mgmt Supported | Reserved | Streaming Supported | Buffer Mgmt Supported | ACK/NAK  <br> Supported | Async Supported | Sync Supported |
| <br> 5 - 15 |  |  |  | <br> Reserved  <br> |  |  |  |  |

| • | Feature Code | = 0x0001 |
| --- | --- | --- |
| • | Version | = 0x1 or any version that supports the defined features in this SSC |
| • | Length | = 0x0C |
| • | ComID Mgmt Supported | = VU |
| • | Streaming Supported | = 1 |
| • | Buffer Mgmt Supported | = VU |
| • | ACK/NACK Supported | = VU |
| • | Async Supported | = VU |
| • | Sync Supported | = 1 |

#### 3.1.1.3 Locking Feature (Feature Code = 0x0002)

** = the present current state of the respective feature

**Table 5 Level 0 Discovery - Locking Feature Descriptor**

| Bit Byte | 7 | 6 |  | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) |  |  |  | Feature Code (0x0002) |  |  |  |  |
| 1 |  |  |  |  |  |  |  |  | (LSB) |
| 2 |  |  | Version |  |  |  | Reserved |  |  |
| 3 |  |  |  |  | Length |  |  |  |  |
| 4 | Reserved |  |  | MBR Done | MBR  <br> Enabled | Media Encryption | Locked | Locking Enabled | Locking Supported |
| 5 - 15 |  |  |  |  | Reserved |  |  |  |  |

| • | Feature Code | = 0x0002 |
| --- | --- | --- |
| • | Version | = 0x1 or any version that supports the defined features in this SSC |
| • | Length | = 0x0C |
| • | MBR Done | = ** |
| • | MBR Enabled | = ** |
| • | Media Encryption | = 1 |
| • | Locked | = ** |
| • | Locking Enabled | = See 3.1.1.3.1 |
| • | Locking Supported | = 1 |

##### 3.1.1.3.1 LockingEnabled Definition

The definition of the LockingEnabled bit is changed from [2] as follows:

The LockingEnabled bit SHALL be set to one if an SP that incorporates the Locking template is any state other than Nonexistent or Manufactured-Inactive; otherwise the LockingEnabled bit SHALL be set to zero.

#### 3.1.1.4 Opalite SSC Feature (Feature Code = 0x0301)

**Table 6 Level 0 Discovery - Opalite SSC Feature Descriptor**

| Bit Byte | 7 | 6 |  | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) |  | Feature Code (0x0301) |  |  |  |  |  |  |
| 1 |  |  |  |  |  |  |  |  | (LSB) |
| 2 |  |  | Version |  |  | Reserved |  |  |  |
| 3 |  |  | Length |  |  |  |  |  |  |
| 4 | (MSB) |  | Base ComID |  |  |  |  |  |  |
| 5 |  |  |  |  |  |  |  |  | (LSB) |
| 6 | (MSB) |  | Number of ComIDs |  |  |  |  |  |  |
| 7 |  |  |  |  |  |  |  |  | (LSB) |
| 8-12 |  |  | Reserved for future common SSC parameters |  |  |  |  |  |  |
| 13 |  |  | Initial C_PIN_SID PIN Indicator |  |  |  |  |  |  |
| 14 |  |  | Behavior of C_PIN_SID PIN upon TPer Revert |  |  |  |  |  |  |
| 15-19 |  |  | Reserved for future common SSC parameters |  |  |  |  |  |  |

- Feature Code = 0x0301
- Version = 0x1 or any version that supports the defined features in this SSC
- Length = 0x10
- Base ComID = VU
- Number of ComIDs = 0x0001 (minimum value)
- Initial C_PIN_SID PIN Indicator = 0x00 o 0x00 = The initial C_PIN_SID PIN value is equal to the C_PIN_MSID PIN value
- Behavior of C_PIN_SID PIN upon TPer Revert = 0x00 o 0x00 = The C_PIN_SID PIN value becomes the value of the C_PIN_MSID PIN column after successful invocation of Revert on the Admin SP’s object in the SP table

If an Opalite SSC implementation is compatible with [5], the SD SHOULD also report the SSC Feature Descriptor as defined in [5]. If an Opalite SSC implementation is compatible with [6], the SD SHOULD also report the SSC Feature Descriptor as defined in [6].

## 3.2 Security Protocol 2 Support

### 3.2.1 ComID Management

ComID management support is reported in Level 0 Discovery. Statically allocated ComIDs are also discoverable via the Level 0 Discovery response.

### 3.2.2 Stack Protocol Reset (M)

An Opalite SSC compliant SD SHALL support the Stack Protocol Reset command. Refer to [2] for details.

### 3.2.3 TPER_RESET command (M)

If the TPER_RESET command is enabled, it SHALL cause the following before the TPer accepts the next IFSEND or IF-RECV command:

1. all dynamically allocated ComIDs SHALL return to the Inactive state;
2. all open sessions SHALL be aborted on all ComIDs;
3. all uncommitted transactions SHALL be aborted on all ComIDs;
4. the synchronous protocol stack for all ComIDs SHALL be reset to its initial state
5. all TCG command and response buffers SHALL be invalidated for all ComIDs;
6. all related method processing occurring on all ComIDs SHALL be aborted;
7. TPer’s knowledge of the host’s communications capabilities, on all ComIDs, SHALL be reset to the initial minimum assumptions defined in [2] or the TPer’s SSC definition;
8. the values of the ReadLocked and WriteLocked columns SHALL be set to True for all Locking SP’s Locking objects that contain the Programmatic enumeration value in the LockedOnReset column;
9. the value of the Done column of the Locking SP’s MBRControl table SHALL be set to False, if the DoneOnReset column contains the Programmatic enumeration value.

The TPER_RESET command is delivered by the transport IF-SEND command. If the TPER_RESET command is enabled, the TPer SHALL accept and acknowledge it at the interface level. If the TPER_RESET command is disabled, the TPer SHALL abort it at the interface level with the “Other Invalid Command Parameter” status (see [4]). There is no IF-RECV response to the TPER_RESET command.

The TPER_RESET command is defined in Table 7.

The Transfer Length SHALL be non-zero. All data transferred SHALL be ignored. **Table 7 TPER_RESET Command**

| FIELD | VALUE |
| --- | --- |
| Command | IF-SEND |
| Protocol ID | 0x02 |
| Transfer Length | Non-zero |
| ComID | 0x0004 |

## 3.3 Communications

### 3.3.1 Communication Properties

The TPer SHALL support the minimum communication buffer size as defined in Section 4.1.1.1. For each ComID, the physical buffer size SHALL be reported to the host via the Properties method.

The TPer SHALL terminate any IF-SEND command whose transfer length is greater than the reported MaxComPacketSize size for the corresponding ComID. For details, reference “Invalid Transfer Length parameter on IF-SEND” in [4].

Data generated in response to methods contained within an IF-SEND command payload subpacket (including the required ComPacket / Packet / Subpacket overhead data) SHALL fit entirely within the response buffer. If the method response and its associated protocol overhead do not fit completely within the response buffer, the TPer

1. SHALL terminate processing of the IF-SEND command payload,
2. SHALL NOT return any part of the method response if the Sync Protocol is being used, and
3. SHALL return an empty response list with a TCG status code of RESPONSE_OVERFLOW in that method’s response status list.

### 3.3.2 Supported Security Protocols

The TPer SHALL support:

- IF-RECV commands with a Security Protocol values of 0x00, 0x01, 0x02.
- IF-SEND commands with a Security Protocol values of 0x01, 0x02.

### 3.3.3 ComIDs

For the purpose of communication using Security Protocol 0x01, the TPer SHALL:

- support at least one statically allocated ComID for Synchronous Protocol communication.
- have the ComID Extension values = 0x0000 for all statically allocated ComIDs.
- keep all statically allocated ComIDs in the Active state.

When the TPer receives an IF-SEND or IF-RECV with an inactive or unsupported ComID, the TPer SHALL either:

- terminate the command as defined in [4] with “Other Invalid Command parameter”, or
- follow the requirements defined in [2] for “Inactive or Unsupported ComID parameter on IF-SEND” or “Inactive or Unsupported ComID parameter on IF-RECV”.

ComIDs SHALL be assigned based on the allocation presented in Table 8

**Table 8 ComID Assignments**

| ComID | Description |
| --- | --- |
| 0x0000 | Reserved |
| 0x0001 | Level 0 Device Discovery |
| 0x0002-0x0003 | Reserved for TCG |
| 0x0004 | TPER_RESET command |
| 0x0005-0x07FF | Reserved for TCG |
| 0x0800-0x0FFF | Vendor Unique |
| 0x1000-0xFFFF | ComID management (Protocol ID=0x01 and 0x02) |

### 3.3.4 Synchronous Protocol

The TPer SHALL support the Synchronous Protocol. Refer to [2] for details.

#### 3.3.4.1 Payload Encoding

##### 3.3.4.1.1 Stream Encoding Modifications

The TPer SHALL support tokens listed in Table 9. If an unsupported token is encountered, the TPer SHALL treat this as a streaming protocol violation and return an error per the definition in section 3.3.4.1.3.

**Table 9 Supported Tokens**

| Acronym | Meaning |
| --- | --- |
|  | Tiny atom |
|  | Short atom |
|  | Medium atom |
|  | Long atom |
| SL | Start List |
| EL | End List |
| SN | Start Name |
| EN | End Name |
| CALL | Call |
| EOD | End of Data |
| EOS | End of session |
| ST | Start transaction |
| ET | End of transaction |
| MT | Empty atom |

The TPer SHALL support the above token atoms with the B bit set to 0 or 1 and the S bit set to 0.

##### 3.3.4.1.2 TCG Packets

Within a single IF-SEND/IF-RECV command, the TPer SHALL support a ComPacket containing one Packet, which contains one Subpacket. The Host MAY discover TPer support of capabilities beyond this requirement in the parameters returned in response to a Properties method.

##### 3.3.4.1.3 Payload Error Response

The TPer SHALL respond according to the following rules if it encounters a streaming protocol violation:

- If the error is on Session Manager or is such that the TPer cannot resolve a valid session ID from the payload (i.e. errors in the ComPacket header or Packet header), then the TPer SHALL discard the payload and immediately transition to the “Awaiting IF-SEND” state.
- If the error occurs after the TPer has resolved the session ID, then the TPer SHALL abort the session and MAY prepare a CloseSession method for retrieval by the host.

### 3.3.5 Storage Device Resets

#### 3.3.5.1 Interface Resets

Interface resets that generate TCG reset events are defined in [4]. Interface initiated TCG reset events SHALL result in:

1. All open sessions SHALL be aborted;
2. All uncommitted transactions SHALL be aborted;
3. All pending session startup activities SHALL be aborted;
4. All TCG command and response buffers SHALL be invalidated;
5. All related method processing SHALL be aborted;
6. For each ComID, the state of the synchronous protocol stack SHALL transition to “Awaiting IF-SEND” state;
7. No notification of these events SHALL be sent to the host.

#### 3.3.5.2 TCG Reset Events

Table 10 replaces the definition of TCG reset_types that are defined in [2]: **Table 10 reset_types**

| Enumeration value | Associated Value |
| --- | --- |
| 0 | Power Cycle |
| 1 | Hardware |
| 2 | HotPlug |
| 3 | Programmatic |
| 4-15 | Reserved |
| 16-31 | Vendor Unique |

### 3.3.6 Protocol Stack Reset Commands (M)

An IF-SEND containing a Protocol Stack Reset Command SHALL be supported.

Refer to [2] for details.

# 4 Opalite SSC-compliant Functions and SPs

## 4.1 Session Manager

### 4.1.1 Methods

#### 4.1.1.1 Properties (M)

An Opalite compliant SD SHALL support the Properties method. The requirements for support of the various TPer and Host properties, and the requirements for their values, are shown in Table 11. **Table 11 Properties Requirements**

| Property Name | TPer Property Requirements and Values Reported | Host Property Requirements and Values Accepted |
| --- | --- | --- |
| MaxComPacketSize | (M)  <br> 2048 minimum | (M)  <br> Initial Assumption: 2048  <br> Minimum allowed: 2048  <br> Maximum allowed: VU |
| MaxResponseComPacketSize | (M)  <br> 2048 minimum | (N)  <br> Although this is a legal host property, there is no requirement for the TPer to use it.  The TPer MAY ignore this host property and not list it in the HostProperties result of the Properties method response. |
| MaxPacketSize | (M)  <br> 2028 minimum | (M)  <br> Initial Assumption: 2028  <br> Minimum allowed: 2028  <br> Maximum allowed: VU |
| MaxIndTokenSize | (M)  <br> 1992 minimum | (M)  <br> Initial Assumption: 1992  <br> Minimum allowed: 1992  <br> Maximum allowed: VU |
| MaxPackets | (M) 1 minimum | (M)  <br> Initial Assumption: 1  <br> Minimum allowed: 1  <br> Maximum allowed: VU |
| MaxSubpackets | (M) 1 minimum | (M)  <br> Initial Assumption: 1  <br> Minimum allowed: 1  <br> Maximum allowed: VU |
| MaxMethods | (M) 1 minimum | (M)  <br> Initial Assumption: 1  <br> Minimum allowed: 1  <br> Maximum allowed: VU |
| MaxSessions | (M) 1 minimum | N/A – not a host property |
| MaxAuthentications | (M) 2 minimum | N/A – not a host property |
| MaxTransactionLimit | (M) 1 minimum | N/A – not a host property |
| DefSessionTimeout | (M)  <br> VU | N/A – not a host property |

#### 4.1.1.2 StartSession (M)

An Opalite-compliant SD SHALL support the following parameters for the StartSession method:

- HostSessionID
- SPID
- Write = support for “True" is (M), support for "False" is (N)
- HostChallenge
- HostSigningAuthority

#### 4.1.1.3 SyncSession (M)

An Opalite-compliant SD SHALL support the following parameters for the SyncSession method:

- HostSessionID
- SPSessionID

#### 4.1.1.4 CloseSession (O)

An Opalite-Compliant SD MAY support the CloseSession method.

## 4.2 Admin SP

The Admin SP includes the Base Template and the Admin Template.

### 4.2.1 Base Template Tables

All tables included in the following subsections are mandatory.

#### 4.2.1.1 SPInfo (M)

**Table 12 Admin SP - SPInfo Table Preconfiguration**

| UID | SPID | Name | Size | SizeInUse | SPSessionTimeout | Enabled |
| --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 02  <br> 00 00 00 01 | 00 00 02 05  <br> 00 00 00 01 | “Admin” |  |  |  | T |

#### 4.2.1.2 SPTemplates (M)

*ST1 = this version number or any version number that complies with this SSC. **Table 13 Admin SP - SPTemplates Table Preconfiguration**

| UID | TemplateID | Name | Version |
| --- | --- | --- | --- |
| 00 00 00 03  <br> 00 00 00 01 | 00 00 02 04 00 00 00 01 | “Base” | 00 00 00 02 *ST1 |
| 00 00 00 03  <br> 00 00 00 02 | 00 00 02 04 00 00 00 02 | “Admin” | 00 00 00 02 *ST1 |

#### 4.2.1.3 Table (M)

Refer to section 5.3 for a description and requirements of the MandatoryWriteGranularity and RecommendedAccessGranularity columns.

**Table 14 Admin SP - Table Table Preconfiguration**

| UID | Name | CommonName | TemplateID | Kind | Column | NumColumns | Rows | RowsFree | RowBytes | LastID | MinSize | MaxSize | MandatoryWrite Granularity | RecommendedAccess Granularity |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 01  <br> 00 00 00 01 | “Table” |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 02 | “SPInfo” |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 03 | “SPTemplates” |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 06 | "MethodID" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 07 | "AccessControl" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 08 | "ACE" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 09 | "Authority" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 0B | "C_PIN" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 02 01 | "TPerInfo" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 02 04 | "Template" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 02 05 | "SP" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |

*Begin Informative Content*

[2] states, “The Table table in the Admin SP includes a row for each table that the TPer supports, in addition to a row for each table that exists in the Admin SP.” However, the Opalite SSC requires only the tables from the Admin SP to be included in the Admin SP’s Table table, as indicated in Table 14. *End Informative Content*

#### 4.2.1.4 MethodID (M)

*MT1 = refer to section 5.1.2 for details on the requirements for supporting Revert.

*MT2 = refer to section 5.1.1 for details on the requirements for supporting Activate.

.

**Table 15 Admin SP - MethodID Table Preconfiguration**

| UID | Name | CommonName | TemplateID |
| --- | --- | --- | --- |
| 00 00 00 06  <br> 00 00 00 08 | "Next" |  |  |
| 00 00 00 06  <br> 00 00 00 0D | "GetACL" |  |  |
| 00 00 00 06  <br> 00 00 00 16 | "Get" |  |  |
| 00 00 00 06  <br> 00 00 00 17 | "Set" |  |  |
| 00 00 00 06  <br> 00 00 00 1C | "Authenticate" |  |  |
| 00 00 00 06  <br> 00 00 02 02  <br>        *MT1 | "Revert" |  |  |
| 00 00 00 06  <br> 00 00 02 03  <br> *MT2 | "Activate" |  |  |
| 00 00 00 06  <br> 00 00 06 01 | “Random” |  |  |

#### 4.2.1.5 AccessControl (M)

The following table contains Optional rows identified by (O)

*AC1 = TT TT TT TT is a shorthand for the LSBs of the Table object UIDs

*AC2 = TT TT TT TT is a shorthand for the LSBs of the SPTemplates object UIDs

*AC3 = TT TT TT TT is a shorthand for the LSBs of the MethodID object UIDs

*AC4 = TT TT TT TT is a shorthand for the LSBs of the ACE object UIDs

*AC5 = TT TT TT TT is a shorthand for the LSBs of the Authority object UIDs

*AC6 = TT TT TT TT is a shorthand for the LSBs of the Template object UIDs

*AC7 = TT TT TT TT is a shorthand for the LSBs of the SP object UIDs

*AC8 = refer to section 5.1.2 for details on the requirements for supporting Revert

*AC9 = refer to section 5.1.1 for details on the requirements for supporting Activate

Notes:

- The InvokingID, MethodID and GetACLACL columns are a special case. Although they are marked as Read-Only with fixed access control, the access control for invocation of the Get method is (N).
- The ACL column is readable only via the GetACL method.

**Table 16 Admin SP - AccessControl Table Preconfiguration**

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Table |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 01 00 00 00 00 00 00 | Table | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 01 00 00 TT TT TT TT *AC1 | TableObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| SPInfo |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 02 00 00 00 01 00 00 | SPInfoObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| SPTemplates |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 03 00 00 00 00 00 00 | SPTemplates | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 03 00 00 TT TT TT TT *AC2 | SPTemplatesObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| MethodID |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 06 00 00 00 00 00 00 | MethodID | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 06 00 00 TT TT TT TT *AC3 | MethodIDObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ACE |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 00 00 | ACE | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 TT TT TT TT *AC4 | ACEObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Authority |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 00 00 00 | Authority | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 TT TT TT TT *AC5 | AuthorityObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| C_PIN |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 00 00 00 | C_PIN | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 00 00 | C_PIN_SID | Get |  | ACE_C_PIN_SID_Get_NOPIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 00 00 0B 00 01 00 00 | C_PIN_SID | Set |  | ACE_C_PIN_SID_Set_PIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 02 00 84 | C_PIN_MSID | Get |  | ACE_C_PIN_MSID_Get_PIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| TPerInfo |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 01 00 02 00 01 03 00 | TPerInfoObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 01 00 02 00 01 03 00 | TPerInfoObj | Set |  | ACE_TPerInfo_Set_ProgrammaticResetEnable |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Template |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
|  |  | 00 04 00 02 00 00 00 00 | Template | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 04 00 02 TT TT TT TT *AC6 | TemplateObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| SP |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 00 00 01 00 00 | ThisSP | Authenticate |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 00 00 01 00 00 | ThisSP | Random |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 02 05 00 00 00 00 | SP | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 02 05 TT TT TT TT *AC7 | SPObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC8 |  | 00 00 02 05 TT TT TT TT *AC7 | SPObj | Revert |  | ACE_SP_SID |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC9 |  | 00 00 02 05 TT TT TT TT *AC7 | SPObj | Activate |  | ACE_SP_SID |  |  |  | ACE_Anybody |  |  |  |  |  |  |

#### 4.2.1.6 ACE (M)

*ACE1 = This row is (M) if the TPer supports either Activate or Revert, and (N) otherwise. **Table 17 Admin SP - ACE Table Preconfiguration**

| Table Association - Informative text | UID | Name | CommonName | BooleanExpr | Columns |
| --- | --- | --- | --- | --- | --- |
| BaseACEs |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 00 00 01 | "ACE_Anybody" |  | Anybody | All |
| C_PIN |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 00 8C 02 | "ACE_C_PIN_SID_Get_NOPIN" |  | Admins OR SID | UID, CharSet, TryLimit, Tries, Persistence |
|  | 00 00 00 08  <br> 00 00 8C 03 | "ACE_C_PIN_SID_Set_PIN" |  | SID | PIN |
|  | 00 00 00 08  <br> 00 00 8C 04 | "ACE_C_PIN_MSID_Get_PIN" |  | Anybody | UID, PIN |
| TPerInfo |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 00 03 | "ACE_TPerInfo_Set_ProgrammaticR esetEnable" |  | SID | ProgrammaticResetEnable |
| SP |  |  |  |  |  |
| *ACE1 | 00 00 00 08  <br> 00 03 00 02 | "ACE_SP_SID" |  | SID | All |

#### 4.2.1.7 Authority (M)

**Table 18 Admin SP - Authority Table Preconfiguration**

| UID | Name | CommonName | IsClass | Class | Enabled | Secure | HashAndSign | PresentCertificate | Operation | Credential | ResponseSign | ResponseExch | ClockStart | ClockEnd | Limit | Uses | Log | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 09  <br> 00 00 00 01 | "Anybody" |  | F | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 00 00 02 | "Admins" |  | T | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 00 00 06 | "SID" |  | F | Null | T | None | None | F | Password | C_PIN_SID | Null | Null |  |  |  |  |  |  |

#### 4.2.1.8 C_PIN (M)

**Table 19 Admin SP - C_PIN Table Preconfiguration**

| UID | Name | CommonName | PIN | CharSet | TryLimit | Tries | Persistence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 0B 00 00 00 01 | "C_PIN_SID" |  | MSID | Null | VU | VU | FALSE |
| 00 00 00 0B 00 00 84 02 | "C_PIN_MSID" |  | MSID |  |  |  |  |

The PIN column value of C_PIN_SID is set to the PIN column value of C_PIN_MSID in OFS.

### 4.2.2 Base Template Methods

Refer to section 4.2.1.4 for supported methods.

### 4.2.3 Admin Template Tables

#### 4.2.3.1 TPerInfo (M)

The TPerInfo table has the following columns, in addition to those defined in [2]: **Table 20 Admin SP – TPerInfo Columns**

| Column Number | Column Name | IsUnique | Colum Type |
| --- | --- | --- | --- |
| 0x08 | ProgrammaticResetEnable |  | boolean |

• **ProgrammaticResetEnable**

This column indicates whether support for programmatic resets is enabled or not. If ProgrammaticResetEnable is TRUE, then the TPER_RESET command is enabled. If ProgrammaticResetEnable is FALSE, then the TPER_RESET command is not enabled. This column is readable by Anybody and modifiable by the SID authority.

*TP1 = the value in the GUDID column SHALL comply with the format defined in [2].

*TP2 = this version or any version that supports the defined features in this SSC.

*TP3 = the SSC column is a list of names and SHALL have “Opalite” as one of the list elements.

**Table 21 Admin SP - TPerInfo Table Preconfiguration**

| U ID | Bytes | GUDID | Generation | Firmware Version | ProtocolVersion | SpaceForIssuance | SSC | ProgrammaticResetEnable |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 02 01  <br> 00 03 00 01 |  | VU *TP1 |  |  | 1 *TP2 |  | [“Opalite”] *TP3 | FALSE |

#### 4.2.3.2 Template (M)

**Table 22 Admin SP - Template Table Preconfiguration**

| UID | Name | Revision Number | Instances | MaxInstances |
| --- | --- | --- | --- | --- |
| 00 00 02 04  <br> 00 00 00 01 | "Base" | 1 | VU | VU |
| 00 00 02 04  <br> 00 00 00 02 | "Admin" | 1 | 1 | 1 |
| 00 00 02 04  <br> 00 00 00 06 | "Locking" | 1 | 1 | 1 |

#### 4.2.3.3 SP (M)

*SP1 = this row only exists in the Admin SP's OFS when the Locking SP is created by the manufacturer.

**Table 23 Admin SP - SP Table Preconfiguration**

| UID | Name | ORG | EffectiveAuth | DateOfIssue | Bytes | LifeCycle | Frozen |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 02 05  <br> 00 00 00 01 | "Admin" |  |  |  |  | Manufactured | FALSE |
| 00 00 02 05  <br> 00 00 00 02  <br>        *SP1 | "Locking" |  |  |  |  | Manufactured-Inactive | FALSE |

### 4.2.4 Admin Template Methods

Refer to section 4.2.1.4 for supported methods.

### 4.2.5 Crypto Template Tables

An Opalite SSC compliant SD is not required to support any Crypto template tables.

### 4.2.6 Crypto Template Methods

Refer to section 4.2.1.4 for supported methods.

#### 4.2.6.1 Random

The TPer SHALL implement the Random method with the constraints stated in this subsection. TPer support of the following parameters is mandatory:

• Count

Attempts to use unsupported parameters SHALL result in a method failure response with TCG status

INVALID_PARAMETER. The TPer SHALL support Count parameter values less than or equal to 32.

## 4.3 Locking SP

### 4.3.1 Base Template Tables

All tables defined with (M) in section titles are mandatory.

#### 4.3.1.1 SPInfo (M)

**Table 24 Locking SP - SPInfo Table Preconfiguration**

| UID | SPID | Name | Size | SizeInU se | SPSessionTimeout | Enabled |
| --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 02  <br> 00 00 00 01 | 00 00 02 05  <br> 00 00 00 02 | "Locking" |  |  |  | T |

#### 4.3.1.2 SPTemplates (M)

*SP1 = this version number or any number that supports the defined features in this SSC **Table 25 Locking SP - SPTemplates Table Preconfiguration**

| UID | TemplateID | Name | Version |
| --- | --- | --- | --- |
| 00 00 00 03  <br> 00 00 00 01 | 00 00 02 04 00 00 00 01 | "Base" | 00 00 00 02 *SP1 |
| 00 00 00 03  <br> 00 00 00 02 | 00 00 02 04 00 00 00 06 | "Locking" | 00 00 00 02 *SP1 |

#### 4.3.1.3 Table (M)

*TT1 = only one of the two K_AES* table is required

Refer to section 5.3 for a description and requirements of the MandatoryWriteGranularity and RecommendedAccessGranularity columns.

**Table 26 Locking SP - Table Table Preconfiguration**

| UID | Name | CommonName | TemplateID | Kind | Column | NumColumns | Rows | RowsFree | RowBytes | LastID | MinSize | MaxSize | MandatoryWrite Granularity | RecommendedAccess Granularity |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 01  <br> 00 00 00 01 | "Table" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 02 | "SPInfo" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 03 | "SPTemplates" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 06 | "MethodID" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 07 | "AccessControl" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 08 | "ACE" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 09 | "Authority" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 0B | "C_PIN" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 1D | "SecretProtect" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 01 | "LockingInfo" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 02 | "Locking" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 03 | "MBRControl" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 04 | "MBR" |  |  | Byte |  |  | 00000 |  |  |  |  |  | VU | VU |
| 00 00 00 01  <br> 00 00 08 05  <br> *TT1 | "K_AES_128” |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 06  <br> *TT1 | "K_AES_256" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 10 01 | "DataStore" |  |  | Byte |  |  |  |  |  |  |  |  | VU | VU |

#### 4.3.1.4 Type (N)

The Type table is (N) by Opalite. The following types as defined by [2] SHALL meet the following requirements:

- The "boolean_ACE" type (00000005 000040E) SHALL include the OR Boolean operator.
- The "AC_element" type (00000005 00000801) SHALL support at least 4 entries (2 User authorities, 1 Admin authority, and 1 Boolean operator).

#### 4.3.1.5 MethodID (M)

*MT1 = refer to section 5.1.3 for details on the requirements for supporting RevertSP.

**Table 27 Locking SP - MethodID Table Preconfiguration**

| UID | Name | CommonName | TemplateID |
| --- | --- | --- | --- |
| 00 00 00 06  <br> 00 00 00 08 | "Next" |  |  |
| 00 00 00 06  <br> 00 00 00 0D | "GetACL" |  |  |
| 00 00 00 06  <br> 00 00 00 10 | "GenKey" |  |  |
| 00 00 00 06  <br> 00 00 00 11  <br> *MT1 | "RevertSP" |  |  |
| 00 00 00 06  <br> 00 00 00 16 | "Get" |  |  |
| 00 00 00 06  <br> 00 00 00 17 | "Set" |  |  |
| 00 00 00 06  <br> 00 00 00 1C | "Authenticate" |  |  |
| 00 00 00 06  <br> 00 00 06 01 | “Random” |  |  |

#### 4.3.1.6 AccessControl (M)

*AC1 = refer to section 5.1.3 for details on the requirements for supporting RevertSP

*AC2 = TT TT TT TT is a shorthand for the LSBs of the Table object UIDs

*AC3 = TT TT TT TT is a shorthand for the LSBs of the SPTemplates object UIDs

*AC4 = TT TT TT TT is a shorthand for the LSBs of the MethodID object UIDs

*AC5 = TT TT TT TT is a shorthand for the LSBs of the ACE object UIDs

*AC6 = only K_AES_128 or K_AES_256 related rows mandatory

*AC7 = TT TT TT TT is a shorthand for the LSB of the Authority object UIDs *AC8 = TT TT TT TT is a shorthand for the LSBs of the SecretProtect object UIDs Notes:

- The InvokingID, MethodID and GetACLACL columns are a special case. Although they are marked as Read-Only with fixed access control, the access control for invocation of the Get method is (N).
- The ACL column is readable only via the GetACL method.

**Table 28 Locking SP - AccessControl Table Preconfiguration**

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| SP |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 00 00 01 00 00 | ThisSP | Authenticate |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 00 00 01 00 00 | ThisSP | Random |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC1 |  | 00 00 00 00 00 01 00 00 | ThisSP | RevertSP |  | ACE_Admin |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Table |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 01 00 00 00 00 00 00 | Table | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 01 00 00 TT TT TT TT *AC2 | TableObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| SPInfo |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 02 00 00 00 01 00 00 | SPInfoObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| SPTemplates |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 03 00 00 00 00 00 00 | SPTemplates | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| <br> |  | 00 03 00 00 TT TT TT TT *AC3 | SPTemplatesObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| MethodID |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 06 00 00 00 00 | MethodID | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| <br> |  | 00 06 00 00 TT TT TT TT *AC4 | MethodIDObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| ACE |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 00 00 | ACE | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| <br> |  | 00 08 00 00 TT TT TT TT *AC5 | ACEObj | Get |  | ACE_ACE_Get_All |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 03 80 | ACE_ACE_Get_All | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 08 00 00 00 00 03 90 | ACE_Authority_Get_All | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 03 01 A8 | ACE_C_PIN_User1_Set_PIN | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 03 02 A8 | ACE_C_PIN_User2 _Set_PIN | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC6 |  | 00 08 00 00 00 00 03 B0 | ACE_K_AES_128_GlobalRange_GenKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| *AC6 |  | 00 08 00 00 00 00 03 B8 | ACE_K_AES_256_GlobalRange_GenKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 03 D0 | ACE_Locking_GlobalRange_Get_ RangeStartToActiveKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 03 E0 | ACE_Locking_GlobalRange_Set_RdLocked | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 08 00 00 00 00 03 E8 | ACE_Locking_GlobalRange_Set_WrLocked | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 01 03 F8 | ACE_MBRControl_Set_DoneToDOR | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 03 00 FC | ACE_DataStore_Get_All | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 08 00 00 00 01 03 FC | ACE_DataStore_Set_All | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Authority |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 00 00 00 | Authority | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 TT TT TT TT *AC7 | AuthorityObj | Get |  | ACE_Authority_Get_All |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 01 03 00 | User1 | Set |  | ACE_Authority_Set_Enabled |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 09 00 00 00 0 03 00 2 | User 2 | Set |  | ACE_Authority_Set_Enabled |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| C_PIN |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 00 00 00 | C_PIN | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 01 00 | C_PIN_Admin1 | Get |  | ACE_C_PIN_Admins_Get_All_NOPIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 03 00 | C_PIN_User1 | Get |  | ACE_C_PIN_Admins_Get_All_NOPIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 00 00 0B 00 02 03 00 | C_PIN_User2 | Get |  | ACE_C_PIN_Admins_Get_All_NOPIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 01 00 | C_PIN_Admin1 | Set |  | ACE_C_PIN_Admins_Set_PIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 03 00 | C_PIN_User1 | Set |  | ACE_C_PIN_User1_Set_PIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 02 03 00 | C_PIN_User2 | Set |  | ACE_C_PIN_User2_Set_PIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| SecretProtect |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 00 00 1D 00 00 00 00 | SecretProtect | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 1D TT TT TT TT *AC8 | SecretProtectObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| LockingInfo |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 01 00 08 00 01 00 00 | LockingInfoObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Locking |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 02 00 08 00 00 00 00 | Locking | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 02 00 08 00 01 00 00 | Locking_GlobalRange | Get |  | ACE_Locking_GlobalRange_Get_ RangeStartToActiveKey |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 02 00 08 00 01 00 00 | Locking_GlobalRange | Set |  | ACE_Locking_GlblRng_Admins_Set, ACE_Locking_GlobalRange_Set_RdLocked, ACE_Locking_GlobalRange_Set_WrLocked |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| MBRControl |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 03 00 08 00 01 00 00 | MBRControlObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 03 00 08 00 01 00 00 | MBRControlObj | Set |  | ACE_MBRControl_Admins_Set, ACE_MBRControl_Set_DoneToDOR |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| MBR |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 04 00 08 00 00 00 00 | MBR | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 00 04 08 00 00 00 00 | MBR | Set |  | ACE_Admin |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| K_AES_128 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 05 00 08 00 01 00 00 | K_AES_128_GlobalRange_Key | Get |  | ACE_K_AES_Mode |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 05 00 08 00 01 00 00 | K_AES_128_GlobalRange_Key | GenKey |  | ACE_K_AES_128_GlobalRange_GenKey |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table Association - informative o n ly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| K_AES_256 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 06 00 08 00 01 00 00 | K_AES_256_GlobalRange_Key | Get |  | ACE_K_AES_Mode |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 06 00 08 00 01 00 00 | K_AES_256_GlobalRange_Key | GenKey |  | ACE_K_AES_256_GlobalRange_GenKey |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| DataStore |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 10 00 01 00 00 00 00 | DataStore | Get |  | ACE_DataStore_Get_All |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 01 00 10 00 00 00 00 | DataStore | Set |  | ACE_DataStore_Set_All |  |  |  | ACE_Anybody |  |  |  |  |  |  |

#### 4.3.1.7 ACE (M)

*ACE1 = The TPer SHALL support the values of “Admins” and “Admins OR User1” in the BooleanExpr column of the ACE_C_PIN_User1_Set_PIN ACE. The TPer SHALL fail the Set method invocation with status INVALID_PARAMETER if the host attempts to set a value not supported by the TPer.

*ACE2 = The TPer SHALL support the values of “Admins” and “Admins OR User2” in the BooleanExpr column of the ACE_C_PIN_User2_Set_PIN ACE. The TPer SHALL fail the Set method invocation with status INVALID_PARAMETER if the host attempts to set a value not supported by the TPer.

**Table 29 Locking SP - ACE Table Preconfiguration**

| Table Association - Informative Column | UID | Name | CommonName | BooleanExpr | Columns |
| --- | --- | --- | --- | --- | --- |
| Base ACEs |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 00 00 01 | "ACE_Anybody" |  | Anybody | All |
|  | 00 00 00 08  <br> 00 00 00 02 | "ACE_Admin" |  | Admins | All |
| ACE |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 80 00 | "ACE_ACE_Get_All" |  | Admins | All |
|  | 00 00 00 08  <br> 00 03 80 01 | "ACE_ACE_Set_BooleanExpression" |  | Admins | BooleanExpr |
| Authority |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 90 00 | "ACE_Authority_Get_All" |  | Admins | All |
|  | 00 00 00 08  <br> 00 03 90 01 | "ACE_Authority_Set_Enabled" |  | Admins | Enabled |
| C_PIN |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 A0 00 | "ACE_C_PIN_Admins_Get_All_NOPIN" |  | Admins | UID, CharSet, TryLimit, Tries, Persistence |
|  | 00 00 00 08  <br> 00 03 A0 01 | "ACE_C_PIN_Admins_Set_PIN" |  | Admins | PIN |
|  | 00 00 00 08  <br> 00 03 A8 01 | "ACE_C_PIN_User1_Set_PIN" |  | Admins OR User1 *ACE1 | PIN |
|  | 00 00 00 08  <br> 00 03 A8 02 | "ACE_C_PIN_User2_Set_PIN" |  | Admins OR User2 *ACE2 | PIN |
| K_AES |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 BF FF | "ACE_K_AES_Mode" |  | Anybody | Mode |
| K_AES_128 |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 B0 00 | "ACE_K_AES_128_GlobalRange_  <br> GenKey" |  | Admins | All |
| K_AES_256 |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 B8 00 | "ACE_K_AES_256_GlobalRange_  <br> GenKey" |  | Admins | All |
| Table Association - Informative Column | UID | Name | CommonName | BooleanExpr | Columns |
| Locking |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 D0 00 | "ACE_Locking_GlobalRange_Get_ RangeStartToActiveKey" |  | Admins | RangeStart,  <br> RangeLength,  <br> ReadLockEnabled,  <br> WriteLockEnabled,  <br> ReadLocked,   <br> WriteLocked,  <br> LockOnReset, ActiveKey |
|  | 00 00 00 08  <br> 00 03 E0 00 | "ACE_Locking_GlobalRange_Set_RdLocked" |  | Admins | ReadLocked |
|  | 00 00 00 08  <br> 00 03 E8 00 | "ACE_Locking_GlobalRange_Set_WrLocked" |  | Admins | WriteLocked |
|  | 00 00 00 08  <br> 00 03 F0 00 | "ACE_Locking_GlblRng_Admins_Set" |  | Admins | ReadLockEnabled,  <br> WriteLockEnabled,  <br> ReadLocked,  <br> WriteLocked,  <br> LockOnReset |
| MBRControl |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 F8 00 | "ACE_MBRControl_Admins_Set" |  | Admins | Enable, Done, DoneOnReset |
|  | 00 00 00 08  <br> 00 03 F8 01 | "ACE_MBRControl_Set_DoneToDOR" |  | Admins | Done, DoneOnReset |
| DataStore |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 FC 00 | "ACE_DataStore_Get_All" |  | Admins | All |
|  | 00 00 00 08  <br> 00 03 FC 01 | "ACE_DataStore_Set_All" |  | Admins | All |

#### 4.3.1.8 Authority (M)

**Table 30 Locking SP - Authority Table Preconfiguration**

| UID | Name | CommonName | IsClass | Class | Enabled | Secure | HashAndSign | PresentCertificate | Operation | Credential | ResponseSign | ResponseExch | ClockStart | ClockEnd | Limit | Uses | Log | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 09  <br> 00 00 00 01 | "Anybody" | "" | F | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 00 00 02 | "Admins" | "" | T | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 01 00 01 | "Admin1" | "" | F | Admins | T | None | None | F | Password | C_PIN_Admin1 | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 03 00 00 | "Users" | "" | T | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 03 00 01 | "User1" | "" | F | Users | F | None | None | F | Password | C_PIN_User1 | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 03 00 02 | "User2" | "" | F | Users | F | None | None | F | Password | C_PIN_User2 | Null | Null |  |  |  |  |  |  |

#### 4.3.1.9 C_PIN (M)

1.

**Table 31 Locking SP - C_PIN Table Preconfiguration**

| UID | Name | CommonName | PIN | CharSet | TryLimit | Tries | Persistence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 0B 00 01 00 01 | "C_PIN_Admin1" |  | SID | Null | 0 | 0 | FALSE |
| 00 00 00 0B 00 03 00 01 | "C_PIN_User1" |  | “” | Null | 0 | 0 | FALSE |
| 00 00 00 0B 00 03 00 02 | "C_PIN_User2" |  | “” | Null | 0 | 0 | FALSE |

#### 4.3.1.10 SecretProtect (M)

At least one of the objects shown in Table 32 SHALL be supported

**Table 32 Locking SP - SecretProtect Table Preconfiguration**

| UID | Table | ColumnNumber | ProtectMechanisms |
| --- | --- | --- | --- |
| 00 00 00 1D  <br> 00 00 00 1D | 00 00 00 01  <br> 00 00 08 05  <br> (K_AES_128) | 0x03 | VU |
| 00 00 00 1D 00 00 00 1E | 00 00 00 01  <br> 00 00 08 06  <br> (K_AES_256) | 0x03 | VU |

Note: The “VU” entries in Table 32 indicate that there is no requirement set by this specification as to the value reported. It is NOT a requirement to report the “Vendor Unique” protection_types value.

### 4.3.2 Base Template Methods

Refer to section 4.3.1.5 for supported methods.

### 4.3.3 Crypto Template Tables

An Opalite SSC compliant SD is not required to support any Crypto template tables.

### 4.3.4 Crypto Template Methods

Refer to section 4.3.1.5 for supported methods.

#### 4.3.4.1 Random

Refer to section 4.2.6.1 for additional constraints imposed on the Random method.

### 4.3.5 Locking Template Tables

#### 4.3.5.1 LockingInfo (M)

The LockingInfo table has the following columns, in addition to those defined in [2]:

**Table 33 Locking SP – LockingInfo Columns**

| Column Number | Column Name | IsUnique | Colum Type |
| --- | --- | --- | --- |
| 0x07 | AlignmentRequired |  | boolean |
| 0x08 | LogicalBlockSize |  | uinteger_4 |
| 0x09 | AlignmentGranularity |  | uinteger_8 |
| 0x0A | LowestAlignedLBA |  | uniteger_8 |

- **AlignmentRequired**

This column indicates whether the TPer requires ranges in the Locking table to be aligned. If AlignmentRequired is TRUE, then the TPer requires ranges to be aligned. If AlignmentRequired is FALSE, then the TPer does not require ranges to be aligned.

This column SHALL NOT be modifiable by the host and may be retrieved by Anybody. • **LogicalBlockSize**

This column indicates the number of bytes in a logical block.

This column SHALL NOT be modifiable by the host and may be retrieved by Anybody.

- **AlignmentGranularity**

This column indicates the number of logical blocks in a group, for alignment purposes (see [6] for a detailed explanation).

This column SHALL NOT be modifiable by the host and may be retrieved by Anybody.

- **LowestAlignedLBA**

This column indicates the lowest logical block address that is located at the beginning of an alignment granularity group (see [6] for a detailed explanation).

This column SHALL NOT be modifiable by the host and may be retrieved by Anybody.

| UID | Name | Version | EncryptSupport | MaxRanges | MaxReEncryptions | KeysAvailableCfg | AlignmentRequired | LogicalBlockSize | AlignmentGranularity | LowestAlignedLBA |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 08 01  <br> 00 00 00 01 |  |  | Media Encryption | 0 |  |  |  |  |  |  |

**Table 34 Locking SP - LockingInfo Table Preconfiguration**

#### 4.3.5.2 Locking (M)

*LT1 = The ActiveKey can be a K_AES_128 object reference (UID) or a K_AES_256 object reference (UID) *LT2 = Only a limited set of LockOnReset values is required to be supported by Opalite SSC SDs. Refer to section 4.3.5.2.1 for details.

**Table 35 Locking SP - Locking Table Preconfiguration**

| UID | Name | CommonName | RangeStart | RangeLength | ReadLockEnabled | WriteLockEnabled | ReadLocked | WriteLocked | LockOnReset | ActiveKey | NextKey | ReEncryptState | ReEncyptRequest | AdvKeyMode | VerifyMode | ContOnReset | LastReEncryptLBA | LastReEncState | GeneralStatus |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 08 02  <br> 00 00 00 01 | "Locking_GlobalRange" | "" | 0 | 0 | F | F | F | F | Power Cycle *LT2 | K_AES_128[256]_GlobalRange_Key * LT 1 |  |  |  |  |  |  |  |  |  |

##### 4.3.5.2.1 LockOnReset Restrictions

The TPer SHALL support the following LockOnReset column values:

1. { 0 } (i.e. Power Cycle); and
2. { 0, 3 } (i.e. Power Cycle and Programmatic).

#### 4.3.5.3 MBRControl (M)

*MC1 = Only a limited set of DoneOnReset values is required to be supported by Opalite SSC SDs. Refer to section 4.3.5.3.1 for details.

**Table 36 Locking SP - MBRControl Table Preconfiguration**

| UID | Enable | Done | DoneOnReset |
| --- | --- | --- | --- |
| 00 00 08 03  <br> 00 00 00 01 | False | False | <br> Power Cycle  <br> *MC1  <br> |

##### 4.3.5.3.1 DoneOnReset Restrictions

The TPer SHALL support the following DoneOnReset column values:

1. { 0 } (i.e. Power Cycle); and
2. { 0, 3 } (i.e. Power Cycle and Programmatic).

#### 4.3.5.4 MBR (M)

The MBR minimum size SHALL be 128 MB (0x08000000).

The initial contents of the MBR table SHALL be vendor unique.

#### 4.3.5.5 K_AES_128 or K_AES_256 (M)

At least one of the following two tables SHALL be supported.

*K1 = indirectly writable using the GenKey Method.

**Table 37 Locking SP - K_AES_128 Table Preconfiguration**

| UID | Name | CommonName | Key | Mode |
| --- | --- | --- | --- | --- |
| 00 00 08 05  <br> 00 00 00 01 | "K_AES_128_GlobalRange_Key" |  | VU  *K1 | VU |

**Table 38 Locking SP - K_AES_256 Table Preconfiguration**

| UID | Name | CommonName | Key | Mode |
| --- | --- | --- | --- | --- |
| 00 00 08 06  <br> 00 00 00 01 | "K_AES_256_GlobalRange_Key" |  | VU *K1 | VU |

### 4.3.6 Locking Template Methods

Refer to Section 4.3.1.5 for supported methods.

### 4.3.7 Non Template Tables

#### 4.3.7.1 DataStore (M)

The DataStore is a byte table. It can be used by the host for generic secure data storage. The access control for modification or retrieval of data in the table initially requires a member of the Admins class authority. These access control settings are personalizable. Initial DataStore content value is VU.

# 5 Appendix – SSC Specific Features

## 5.1 Opalite SSC-Specific Methods

### 5.1.1 Activate – Admin Template SP Object Method

Activate is an Opalite SSC-specific method for managing the life cycle of SPs created in manufacturing (Manufactured SP), whose initial life cycle state is “Manufactured-Inactive”.

SPObjectUID.Activate[ ]

=> [ ]

Activate is an object method that operates on objects in the Admin SP’s SP table. The TPer SHALL NOT permit Activate to be invoked on the SP objects of issued SPs.

Invocation of Activate on an SP object that is in the “Manufactured-Inactive” state causes the SP to transition to the “Manufactured” state. Invocation of Activate on an SP in any other life cycle state SHALL complete successfully provided access control is satisfied, and have no effect. The Activate method allows the TPer owner to “turn on” an SP that was created in manufacturing.

This method operates within a Read-Write session to the Admin SP. The SP SHALL be activated immediately after the method returns success if its invocation is not contained within a transaction.

In case of an “Activate Error” (see [4]) Activate SHALL fail with a status of FAIL.The MethodID for Activate SHALL be 00 00 00 06 00 00 02 03.

#### 5.1.1.1 Activate Support

Support for Activate within transactions is (N), and the behavior is out of the scope of this document.

If the Locking SP was created in manufacturing, and its Original Factory State is Manufactured-Inactive (see section 5.2.2), support for Activate on the Locking SP’s object in the SP Table is mandatory.

#### 5.1.1.2 Side effects of Activate

Upon successful activation of an SP that was in the “Manufactured-Inactive” state, the following changes SHALL be made:

- The LifeCycleState column of SP’s object in the Admin SP’s SP table SHALL change to “Manufactured”.
- The current SID PIN (C_PIN_SID) in the Admin SP is copied into the PIN column of Admin1’s C_PIN credential (C_PIN_Admin1) in the activated SP. This allows for taking ownership of the SP with a known PIN credential.
- Any TPer functionality affected by the life cycle state of the SP based on the templates incorporated into it is modified as defined in the appropriate Template reference section of [2], and as defined in the “State transitions for Manufactured SPs” section (section 5.2.2.2) and “State behaviors for Manufactured SPs” section (section 5.2.2.3) of this specification.

### 5.1.2 Revert – Admin Template SP Object Method

Revert is an Opalite SSC-specific method for managing the life cycle of SPs created in manufacturing (Manufactured SP).

SPObjectUID.Revert[ ]

=> [ ]

Revert is an object method that operates on objects in the Admin SP’s SP table. The TPer SHALL NOT permit Revert to be invoked on the SP objects of issued SPs.

Invoking Revert on an SP object causes the SP to revert to its Original Factory State. This method allows the TPer owner (or TPer manufacturer, if access control permits and the Maker authorities are enabled) to remove the SP owner’s ownership of the SP and revert the SP to its Original Factory State.

Invocation of Revert is permitted on Manufactured SPs that are in any life cycle state. Successful invocation of Revert on a Manufactured SP that is in the Manufactured-Inactive life cycle state SHALL have no effect on the SP.

This method operates within a Read-Write session to the Admin SP. The TPer SHALL revert the SP immediately after the method is successfully invoked outside of a transaction. If Revert is invoked on the Admin SP’s object in the SP table, the TPer SHALL abort the session immediately after reporting status of the method invocation if invoked outside of a transaction. The TPer MAY prepare a CloseSession method for retrieval by the host to indicate that the session has been aborted.

The MethodID for Revert SHALL be 00 00 00 06 00 00 02 02.

#### 5.1.2.1 Revert Support

Support for Revert within transactions is (N), and the behavior is out of the scope of this document.

Support for Revert on the Admin SP’s object in the SP table is mandatory. (Note that the OFS of the Admin SP is Manufactured, see 5.2.2).

If the Locking SP was created in manufacturing, support for Revert on the Locking SP’s object in the SP Table is mandatory.

#### 5.1.2.2 Side effects of Revert

Upon successful invocation of the Revert method, the following changes SHALL be made:

- The row in the Admin SP’s SP table that represents this SP SHALL revert to its original factory values.
- The SP itself SHALL revert to its Original Factory State. While reverting to its Original Factory State, the TPer SHALL securely erase all personalization of the SP, and revert the personalized values to their original factory values. The mechanism for secure erasure is implementation-specific. Informative note: Unless already in the Manufactured-Inactive life cycle state, reverting the Locking SP will cause the media encryption keys to be eradicated, which has the side effect of securely erasing all data in the User LBA portion of the SD.
- When Revert is successfully invoked on the SP object for the Admin SP (UID = 00 00 02 05 00 00 00 01), the ***entire TPer*** SHALL revert to its Original Factory State, including all personalization of the Admin SP itself, with the exception of the PIN column value of the C_PIN_SID object. See section **Error! Reference source not found.** for the effects of Revert upon the PIN column value of the C_PIN_SID object. All issued SPs SHALL be deleted, and all Manufactured SPs SHALL revert to Original Factory State. Manufactured SPs that were in the Manufactured-Inactive life cycle state SHALL be unaffected.
- Any TPer functionality affected by the life cycle state of the SP based on the templates incorporated into it is modified as defined in the appropriate Template reference section of [2], and as defined in the “State transitions for Manufactured SPs” section (section 5.2.2.2) and “State behaviors for Manufactured SPs” section (section 5.2.2.3) of this specification.

### 5.1.3 RevertSP – Base Template SP Method

RevertSP is an Opalite SSC-specific method for managing the life cycle of an SP, if it was created in manufacturing (Manufactured SP).

ThisSP.RevertSP[ KeepGlobalRangeKey = boolean ]

=>

[ ]

RevertSP is an SP method in the Base Template.

Invoking RevertSP on an SP SHALL cause it to revert to its Original Factory State. This method allows the SP owner to relinquish control of the SP and revert the SP to its Original Factory State.

This method operates within a Read-Write session to an SP. The TPer SHALL revert the SP immediately after the method is successfully invoked outside of a transaction. Upon completion of reverting the SP, the TPer SHALL report status of the method invocation if invoked outside of a transaction, and then immediately abort the session. The TPer MAY prepare a CloseSession method for retrieval by the host to indicate that the session has been aborted.

The MethodID for RevertSP SHALL be 00 00 00 06 00 00 00 11.

#### 5.1.3.1 RevertSP Support

Support for RevertSP within transactions is (N), and the behavior is out of the scope of this document.

If the Locking SP was created in manufacturing, support for RevertSP on the Locking SP is mandatory.

#### 5.1.3.2 KeepGlobalRangeKey parameter (Locking Template-specific)

The optional **KeepGlobalRangeKey** parameter is a Locking Template-specific optional parameter. This parameter provides a mechanism for the Locking SP to be “turned off” without eradicating the media encryption key for the Global locking range. This allows the TCG management of the SD's locking and media encryption features to be disabled without causing a cryptographic erase of the user data associated with the Global locking range.

When this parameter is present and set to True, the TPer SHALL continue to use the media encryption key associated with the Global locking range after the Locking SP transitions to the “Manufactured-Inactive” state. The following condition SHALL guarantee that the TPer can comply with the request to keep the Global Range’s media encryption key:

o The Global Range is either Read Unlocked or Write Unlocked at the time of invocation of RevertSP If the TPer cannot comply with the request to keep the Global Range’s media encryption key, then the method invocation SHALL fail with status FAIL, and the SP SHALL NOT change life cycle states.

If the Locking SP was created in manufacturing, support for the **KeepGlobalRangeKey** parameter is mandatory for the Locking SP.

The parameter number for **KeepGlobalRangeKey** SHALL be 0x060000.

#### 5.1.3.3 Side effects of RevertSP

Upon successful invocation of the RevertSP method, the following changes SHALL be made:

- The SP’s object in the Admin SP’s SP table SHALL revert to its original factory values.
- The SP itself SHALL revert to its Original Factory State. While reverting to its Original Factory State, the TPer SHALL securely erase all personalization of the SP, and revert the personalized values to their original factory values. The mechanism for secure erasure is implementation-specific. The exception to the secure erasure is the value of the Global Range’s media encryption key (K_AES_{128,256}_GlobalRange_Key) in the Locking SP, if the **KeepGlobalRangeKey** parameter is present and set to True. Informative note: Reverting the Locking SP will cause the media encryption keys to be eradicated (except for the GlobalRange key if the **KeepGlobalRangeKey** parameter is present and set to True), which has the side effect of securely erasing all data in the User LBA portion of the SD.
- Any TPer functionality affected by the life cycle state of the SP based on the templates incorporated into it is modified as defined in the appropriate Template reference section of [2], and as defined in the “State transitions for Manufactured SPs” section (section 5.2.2.2) and “State behaviors for Manufactured SPs” section (section 5.2.2.3) of this specification.

## 5.2 Life Cycle

### 5.2.1 Issued vs. Manufactured SPs

#### 5.2.1.1 Issued SPs

[2] describes the life cycle states for SPs that are created through the issuance process. For Opalite SSCcompliant TPers that support issuance, refer to [2] for the life cycle states and life cycle management.

#### 5.2.1.2 Manufactured SPs

[2] defines the life cycle and life cycle management of Manufactured SPs as implementation-specific.

Opalite SSC-compliant SPs that are created in manufacturing (Manufactured SPs) SHALL NOT have implementation-specific life cycle, and SHALL conform to the life cycle defined in section 5.2.2.

### 5.2.2 Manufactured SP Life Cycle States

The state diagram for Manufactured SPs is shown in Figure 1.

**Figure 1 Life Cycle State Diagram for Manufactured SPs**

Additional state transitions may exist depending on the states supported by the SD and the SP’s Original Factory State. Invoking Revert or RevertSP (see sections 5.1.2 and 5.1.3) on the SP will cause the SP to transition back to its Original Factory State.

The Original Factory State of the Admin SP SHALL be Manufactured. The only state that is mandatory for the Admin SP is Manufactured.

If the Locking SP is a Manufactured SP, its Original Factory State SHALL be Manufactured-Inactive. Support for Locking SP states of Manufactured and Manufactured-Inactive are mandatory. The other states in the state diagram are beyond the scope of this document.

#### 5.2.2.1 State definitions for Manufactured SPs

1. **Manufactured-Inactive**: This is the Original Factory State for SPs that are created in manufacturing, where it is not desirable for the functionality of that SP to be active when the TPer is shipped. All templates that exist in an SP that is in the Manufactured-Inactive state SHALL be counted in the Instances column of the appropriate objects in the Admin SP’s Template table. Sessions cannot be opened to SPs in the Manufactured-Inactive state. Only SPs whose Original Factory State was Manufactured-Inactive can return to the Manufactured-Inactive state.

If the Locking SP is a Manufactured SP, support for the Manufactured-Inactive state is mandatory for the Locking SP.

2. **Manufactured**: This is the standard operational state of a Manufactured SP, and defines the initial required access control settings of an SP based on the Templates incorporated into the SP, prior to personalization.

The Manufactured state is mandatory for the Admin SP.

If the Locking SP is a Manufactured SP, support for the Manufactured state is mandatory for the Locking SP.

#### 5.2.2.2 State transitions for Manufactured SPs

The following sections describe the mandatory and optional state transitions for Opalite SSC-compliant Manufactured SPs.

For the Admin SP, the only transition for which support is mandatory is “ANY STATE to ORIGINAL FACTORY STATE” (5.2.2.2.2). As the only mandatory state for the Admin SP is Manufactured, the only mandatory transition is from Manufactured to Manufactured with the side effect of reverting the entire TPer to its Original Factory State. See section 5.1.2 for details.

If the Locking SP is a Manufactured SP, support for the “ANY STATE to ORIGINAL FACTORY STATE” transition (5.2.2.2.2) is mandatory. Specifically:

- Support for the transition from Manufactured to Manufactured-Inactive is mandatory. This transition is accomplished via the Revert or RevertSP method (see sections 5.1.2 and 5.1.3).
- Support for the “Manufactured-Inactive to Manufactured” transition (5.2.2.2.1) is mandatory. This transition is accomplished via the Activate method (see section 5.1).

##### 5.2.2.2.1 Manufactured-Inactive to Manufactured

Triggers:

- The Activate method (see section 5.1) is successfully invoked on the SP’s object in the Admin SP’s SP table.

Side effects:

- The value in the LifeCycleState column of the SP’s object in the Admin SP’s SP table changes to Manufactured.
- The current SID PIN (C_PIN_SID) in the Admin SP is copied into the PIN column of Admin1’s C_PIN credential (C_PIN_Admin1) in the activated SP. This allows for taking ownership of the SP with a known PIN credential.
- Any functionality enabled by the templates incorporated into the SP becomes active.

When the Locking SP transitions from the Manufactured-Inactive state to the Manufactured state (via invocation of the Activate method), the SD SHALL NOT destroy any user data.

##### 5.2.2.2.2 ANY STATE to ORIGINAL FACTORY STATE

Triggers:

- Revert or RevertSP is successfully invoked on the SP.

Side effects:

- The value in the LifeCycleState column of the SP’s object in the Admin SP’s SP table changes to the value of the SP’s Original Factory State.
- The SP itself reverts to its Original Factory State, as described in the sections 5.1.2 and 5.1.3.
- If the SP’s Original Factory State was Manufactured-Inactive, any functionality enabled by the templates incorporated into the SP becomes inactive.

#### 5.2.2.3 State behaviors for Manufactured SPs

##### 5.2.2.3.1 Manufactured-Inactive

Any functionality enabled by the templates incorporated into the SP is inactive in this state. Sessions cannot be opened to SPs in this state.

When the Locking SP is in the Manufactured-Inactive state, the TCG management of the SD's locking and media encryption features SHALL be disabled.

##### 5.2.2.3.2 Manufactured

Behavior of an SP in the Manufactured state is identical to the behavior of an SP in the Issued state, as described by [2].

When the Locking SP is in the Manufactured state, the TCG management of the SD's locking and media encryption features SHALL be enabled.

### 5.2.3 Type Table Modification

In order to accommodate the additional life cycle states defined in Opalite, the life_cycle_state type SHALL be defined as follows for Opalite:

**Table 39 LifeCycle Type Table Modification**

| UID | Name | Format | Size | Description |
| --- | --- | --- | --- | --- |
| 00 00 00 05  <br> 00 00 04 05 | life_cycle_state | Enumeration_Type,  <br> 0,  <br> 15 |  | Used to represent the current life cycle state.  The valid values are:   <br> 0 = issued, 1 = issued-disabled, 2 = issued-frozen, 3 = issueddisabled-frozen, 4 = issued-failed, 5-7 = reserved, 8 = manufacturedinactive, 9 = manufactured, 10 = manufactured-disabled, 11 = manufactured-frozen, 12 = manufactured-disabled-frozen, 13 = manufactured-failed, 14-15 = reserved |

## 5.3 Byte Table Access Granularity

*Begin Informative Content*

While the general architecture defined in [2] allows data to be written into byte tables starting at any arbitrary byte boundary and with any arbitrary byte length, certain types of storage devices work more efficiently when data is written aligned to a larger block boundary. This section defines extensions to [2] that allow a device to report the restrictions that it enforces when the host invokes the Set method on byte tables. *End Informative Content*

### 5.3.1 Table Table Modification

In order to allow a storage device to report its mandatory and recommended data alignment restrictions when accessing byte tables, the Table table SHALL contain the additional columns shown in Table 40. **Table 40 Table Table Additional Columns**

| Column Number | Column Name | IsUnique | Colum Type |
| --- | --- | --- | --- |
| 0x0D | MandatoryWriteGranularity |  | uinteger_4 |
| 0x0E | RecommendedAccessGranularity |  | uinteger_4 |

#### 5.3.1.1 MandatoryWriteGranularity

This column is used to report the granularity that the storage device enforces when the host invokes the Set method on byte tables.

This column SHALL NOT be modifiable by the host.

##### 5.3.1.1.1 Object Tables

For rows in the Table table that pertain to object tables, the value of this column SHALL be zero.

##### 5.3.1.1.2 Byte Tables

For rows in the Table table that pertain to byte tables, this column indicates the mandatory access granularity (in bytes) for the Set method for the table described in this row of the Table table. The MandatoryWriteGranularity column indicates the alignment requirement for both the access start offset (the Where parameter) and length (number of bytes in the Values parameter).

The value of this column SHALL be less than or equal to the value in the RecommendedAccessGranularity column in the same row of the Table table.

MandatoryWriteGranularity SHALL be less than or equal to 8192.

When the host invokes the Set method on a byte table, if ValidMandatoryGranularity (see Figure 2) is False, then the method SHALL fail with status INVALID_PARAMETER.

If the TPer does not have a requirement on mandatory alignment for the byte table described in a row of the Table table, then its MandatoryWriteGranularity column SHALL be set to 1.

**Figure 2 ValidMandatoryGranularity**

| For the Set method:  <br>    ValidMandatoryGranularity is True if  <br> (x modulo MandatoryWriteGranularity) = 0  <br>   <br>                   and  <br>   <br> (y modulo MandatoryWriteGranularity) = 0  <br>   <br> where:  <br>    x =  the start offset of the Set method   <br>             (i.e., the value of the Where parameter)    y = the number of data bytes being set   <br>             (i.e., the length of the Values parameter) |
| --- |

#### 5.3.1.2 RecommendedAccessGranularity

This column is used to report the granularity that the storage device recommends when the host invokes the Set or Get method on byte tables.

This column SHALL NOT be modifiable by the host.

##### 5.3.1.2.1 Object Tables

For rows in the Table table that pertain to object tables, the value of this column SHALL be zero.

##### 5.3.1.2.2 Byte Tables

For rows in the Table table that pertain to byte tables, this column indicates the recommended access granularity (in bytes) for the Set and Get method for the table described in this row of the Table table. The RecommendedAccessGranularity column indicates the alignment of data for the Set and Get method that allows for optimal Set/Get performance.

If the TPer does not have a recommended alignment for the byte table described in a row of the Table table, then its RecommendedAccessGranularity column SHALL be set to 1.

When the host invokes the Set method on a byte table, if ValidRecommendedGranularity (see Figure 3) is False, then the performance of the TPer MAY be reduced when processing the method.

**Figure 3 ValidRecommendedGranularity for Set**

| For the Set method:  <br>    ValidRecommendedGranularity is True if  <br> (x modulo RecommendedAccessGranularity) = 0  <br>   <br>                   and  <br>   <br> (y modulo RecommendedAccessGranularity) = 0  <br>   <br> where:  <br>    x =  the start offset of the Set method   <br>             (i.e., the value of the Where parameter)    y = the number of data bytes being set   <br>             (i.e., the length of the Values parameter) |
| --- |

When the host invokes the Get method on a byte table, if ValidRecommendedGranularity (see Figure 4) is False, then the performance of the TPer MAY be reduced when processing the method.

**Figure 4 ValidRecommendedGranularity for Get**

For the Get method:

ValidRecommendedGranularity is True if

1. (x modulo RecommendedAccessGranularity) = 0

and

2. (y modulo RecommendedAccessGranularity) = 0

where:

x = the start offset of the Get method

(i.e., the value of the startRow component of the Cellblock parameter) y = the number of data bytes being retrieved

(i.e., the difference of the endRow and startRow components of the Cellblock parameter, plus one)
