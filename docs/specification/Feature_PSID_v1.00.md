**TCG Storage**

**Opal SSC Feature Set: PSID**

**Specification Version 1.00**

**Revision 1.00**

**August 5, 2015**

Contact: admin@trustedcomputinggroup.org

**PUBLISHED**

Copyright © TCG 2015

Copyright © 2015 Trusted Computing Group, Incorporated.

**Disclaimers, Notices, and License Terms**

THIS SPECIFICATION IS PROVIDED "AS IS" WITH NO WARRANTIES WHATSOEVER, INCLUDING ANY WARRANTY OF MERCHANTABILITY, NONINFRINGEMENT, FITNESS FOR ANY PARTICULAR PURPOSE, OR ANY WARRANTY OTHERWISE ARISING OUT OF ANY PROPOSAL, SPECIFICATION OR SAMPLE.

Without limitation, TCG disclaims all liability, including liability for infringement of any proprietary rights, relating to use of information in this specification and to the implementation of this specification, and TCG disclaims all liability for cost of procurement of substitute goods or services, lost profits, loss of use, loss of data or any incidental, consequential, direct, indirect, or special damages, whether under contract, tort, warranty or otherwise, arising in any way out of use or reliance upon this specification or any information herein.

This document is copyrighted by Trusted Computing Group (TCG), and no license, express or implied, is granted herein other than as follows: You may not copy or reproduce the document or distribute it to others without written permission from TCG, except that you may freely do so for the purposes of (a) examining or implementing TCG specifications or (b) developing, testing, or promoting information technology standards and best practices, so long as you distribute the document with these disclaimers, notices, and license terms.

Contact the Trusted Computing Group at [www.trustedcomputinggroup.org](http://www.trustedcomputinggroup.org/) for information on specification licensing through membership agreements.

Any marks and brands contained herein are the property of their respective owners.

**TABLE OF CONTENTS**

[1 INTRODUCTION [1](#introduction)](#introduction)

[1.1 DOCUMENT PURPOSE [1](#document-purpose)](#document-purpose)

[1.2 SCOPE AND INTENDED AUDIENCE [1](#scope-and-intended-audience)](#scope-and-intended-audience)

[1.3 KEY WORDS [1](#key-words)](#key-words)

[1.4 DOCUMENT REFERENCES [1](#document-references)](#document-references)

[1.5 DOCUMENT PRECEDENCE [1](#document-precedence)](#document-precedence)

[1.6 DEPENDENCIES ON OTHER FEATURE SETS [2](#dependencies-on-other-feature-sets)](#dependencies-on-other-feature-sets)

[1.7 INTERACTIONS WITH OTHER FEATURE SETS [2](#interactions-with-other-feature-sets)](#interactions-with-other-feature-sets)

[1.8 TERMINOLOGY [2](#terminology)](#terminology)

[1.9 LEGEND [3](#legend)](#legend)

[2 PSID FEATURE SET OVERVIEW [4](#psid-feature-set-overview)](#psid-feature-set-overview)

[3 SSC SPECIFIC FUNCTIONALITY [5](#ssc-specific-functionality)](#ssc-specific-functionality)

[4 FEATURE SET REQUIREMENTS [6](#feature-set-requirements)](#feature-set-requirements)

[4.1 LEVEL 0 DISCOVERY [6](#level-0-discovery)](#level-0-discovery)

[4.2 ADMIN SP [6](#admin-sp)](#admin-sp)

[4.2.1 PSID Authority (M) [6](#psid-authority-m)](#psid-authority-m)

[4.2.1.1 PSID Authority Object Access Control [6](#psid-authority-object-access-control)](#psid-authority-object-access-control)

[4.2.2 PSID Authority Associated Credential [7](#psid-authority-associated-credential)](#psid-authority-associated-credential)

[4.2.2.1 PSID C_PIN Object Access Control [7](#psid-c_pin-object-access-control)](#psid-c_pin-object-access-control)

[4.2.3 New ACEs [7](#new-aces)](#new-aces)

[4.2.3.1 ACE_C_PIN_Get_PSID_NoPIN [7](#ace_c_pin_get_psid_nopin)](#ace_c_pin_get_psid_nopin)

[4.2.3.1.1 ACE_C_PIN_Get_PSID_NoPIN Access Control [8](#ace_c_pin_get_psid_nopin-access-control)](#ace_c_pin_get_psid_nopin-access-control)

[4.2.3.2 ACE_SP_PSID [8](#ace_sp_psid)](#ace_sp_psid)

[4.2.3.2.1 ACE_SP_PSID Access Control [8](#ace_sp_psid-access-control)](#ace_sp_psid-access-control)

[4.2.4 Performing Device Reversion [8](#performing-device-reversion)](#performing-device-reversion)

[4.3 LOCKING SP [9](#locking-sp)](#locking-sp)

[4.4 ADDITIONAL SPS [9](#additional-sps)](#additional-sps)

|  | Tables |
| --- | --- |
| Table 1 | PSID Authority............................................................................................................................. 6 |
| Table 2 | PSID Authority Access Control Settings ..................................................................................... 6 |
| Table 3 | PSID C_PIN ................................................................................................................................ 7 |
| Table 4 | PSID C_PIN Access Control Settings ......................................................................................... 7 |
| Table 5 | ACE_C_PIN_Get_PSID_NoPIN ACE ......................................................................................... 8 |
| Table 6 | ACE_SP_PSID ACE ................................................................................................................... 8 |
| Table 7 | Device Reversion Access Control Settings ................................................................................ 9 |

## Table of Contents

- [1 Introduction](#1-introduction)
  - [1.1 Document Purpose](#11-document-purpose)
  - [1.2 Scope and Intended Audience](#12-scope-and-intended-audience)
  - [1.3 Key Words](#13-key-words)
  - [1.4 Document References](#14-document-references)
  - [1.5 Document Precedence](#15-document-precedence)
  - [1.6 Dependencies on Other Feature Sets](#16-dependencies-on-other-feature-sets)
  - [1.7 Interactions with Other Feature Sets](#17-interactions-with-other-feature-sets)
  - [1.8 Terminology](#18-terminology)
  - [1.9 Legend](#19-legend)
- [2 PSID Feature Set Overview](#2-psid-feature-set-overview)
- [3 SSC Specific Functionality](#3-ssc-specific-functionality)
- [4 Feature Set Requirements](#4-feature-set-requirements)
  - [4.1 Level 0 Discovery](#41-level-0-discovery)
  - [4.2 Admin SP](#42-admin-sp)
    - [4.2.1 PSID Authority (M)](#421-psid-authority-m)
      - [4.2.1.1 PSID Authority Object Access Control](#4211-psid-authority-object-access-control)
    - [4.2.2 PSID Authority Associated Credential](#422-psid-authority-associated-credential)
      - [4.2.2.1 PSID C_PIN Object Access Control](#4221-psid-c_pin-object-access-control)
    - [4.2.3 New ACEs](#423-new-aces)
      - [4.2.3.1 ACE_C_PIN_Get_PSID_NoPIN](#4231-ace_c_pin_get_psid_nopin)
        - [4.2.3.1.1 ACE_C_PIN_Get_PSID_NoPIN Access Control](#42311-ace_c_pin_get_psid_nopin-access-control)
      - [4.2.3.2 ACE_SP_PSID](#4232-ace_sp_psid)
        - [4.2.3.2.1 ACE_SP_PSID Access Control](#42321-ace_sp_psid-access-control)
    - [4.2.4 Performing Device Reversion](#424-performing-device-reversion)
  - [4.3 Locking SP](#43-locking-sp)
  - [4.4 Additional SPs](#44-additional-sps)

# 1 Introduction

## 1.1 Document Purpose

The Storage Workgroup specifications provide a comprehensive architecture for putting Storage Devices under policy control as determined by the trusted platform host, the capabilities of the Storage Device to conform with the policies of the trusted platform, and the lifecycle state of the Storage Device as a Trusted Peripheral.

## 1.2 Scope and Intended Audience

This specification defines the PSID Feature Set for the Opal Security Subsystem Class (SSC). Any Storage Device that claims Opal SSC PSID Feature Set compatibility SHALL conform to this specification.

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

[2]. Trusted Computing Group (TCG), “TCG Storage Architecture Core Specification”, see [3] for the applicable specification version

[3]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Opal”, Versions 1.00, 2.00,

2.01, or compatible

[4]. Trusted Computing Group (TCG), “TCG Storage Storage Interface Interactions Specification“, see [3] for the applicable specification version

## 1.5 Document Precedence

In the event of conflicting information in this specification and other documents, the precedence for requirements is:

1. This specification and TCG Storage Security Subsystem Class: Opal [3] (these two documents are at the same level of precedence, and SHALL NOT conflict with each other)
2. Storage Interface Interactions Specification [4]
3. TCG Storage Architecture Core Specification [2]

## 1.6 Dependencies on Other Feature Sets

This feature set has no dependencies on other feature sets.

## 1.7 Interactions with Other Feature Sets

This feature set has no interactions with other feature sets.

## 1.8 Terminology

This section provides special definitions that are not defined in the Core Specification. **Table 1 PSID Feature Set Terminology**

| Term | Definition |
| --- | --- |
| PSID | Physical Presence SID |

## 1.9 Legend

The following legend defines SP table cell coloring coding. This color coding is informative only. The table cell content is normative.

**Table 2 SP Table Legend**

| Table  <br> Cell  <br> Legend | R-W | Value | Access Control |  | Comment |
| --- | --- | --- | --- | --- | --- |
| Arial-Narrow | Read-only | Opal SSC specified | Fixed | •  <br> •  <br> • | Cell content is Read-Only.  <br> Access control is fixed.  <br> Value is specified by the Opal  <br> SSC |
| Arial Narrow bold-under | Read-only | VU | Fixed | •  <br> •  <br> • | Cell content is Read-Only.  <br> Access Control is fixed.  <br> Values are Vendor Unique (VU). A minimum or maximum value may be specified. |
| Arial- <br> Narrow | Not Defined | (N) | Not Defined | •  <br> •  <br> • | Cell content content is (N).  <br> Access control is not defined. Any text in table cell is informative only. |
|  |  |  |  | • | A Get MAY omit this column from the method response. |
| Arial Narrow bold-under | Write | Preconfigured,  user personalizable | Preconfigured, user personalizable | •  <br> •  <br> • | Cell content is writable.  <br> Access control is personalizable Get Access Control is not described by this color coding |
| Arial-Narrow | Write | Preconfigured, user personalizable | Fixed | •  <br> •  <br> • | Cell content is writable.  <br> Access control is fixed. Get Access Control is not described by this color coding |

# 2 PSID Feature Set Overview

*Begin Informative Content*

The goal of the PSID Feature Set is to:

|  | Define an authority/credential pair whose C_PIN object’s PIN column value is not discoverable via the interface.  <br> Provide access control settings that permit invocation of the Revert method if the PSID authority has been successfully authenticated. |
| --- | --- |
| The primary use case for this is one where the SID is not known, or manageability of the device has been lost, and the owner wishes to attempt to reset the device to OFS via the Revert method defined in [3].    <br> Because the PSID credential value is specified such that it is not discoverable via the interface, it is necessary that the credential value be delivered via an alternative mechanism.  Examples of these mechanisms include printing the PSID credential on a device label; or including the value as an insert in the device packaging.  The delivery mechanism is vendor unique. End Informative Content |  |

# 3 SSC Specific Functionality

This feature set requires no additional SSC-specific functionality.

# 4 Feature Set Requirements

This section defines the Mandatory (M) and Optional (O) requirements for the PSID Feature Set, when it is implemented in an Opal-compliant device.

## 4.1 Level 0 Discovery

No Level 0 Feature Descriptor is needed for this feature set. Support for the PSID Feature Set can be determined by inspection of the Admin SP’s Authority table.

## 4.2 Admin SP

An Opal-compliant SD that contains the PSID Feature Set SHALL contain the additions to the Admin SP specified in this section, in addition to the Admin SP requirements defined in [3].

### 4.2.1 PSID Authority (M)

The PSID authority SHALL have the characteristics defined in this section.

**Table 1 PSID Authority**

| UID | Name | CommonName | IsClass | Class | Enabled | Secure | HashAndSign | PresentCertif icate | Operation | Credential | ResponseSign | ResponseExch | ClockStart | ClockEnd | Limit | Uses | Log | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 09  <br> 00 01 FF 01 | "PSID" | “PhysicalDrive Owner” | F | Null | T | None | None | F | Password | C_PIN_PSID | Null | Null |  |  |  |  |  |  |

#### 4.2.1.1 PSID Authority Object Access Control

Access control on methods applicable to the PSID authority object are as defined in this section.

**Table 2 PSID Authority Access Control Settings**

| Table Association - informative oly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | Remo veACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Authority |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 01 01 FF | PSID | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |

### 4.2.2 PSID Authority Associated Credential

This section defines the attributes of the credential associated with the PSID authority, C_PIN_PSID. Support for this credential is Mandatory (M) if the PSID authority is supported.

The PIN column value SHALL contain a vendor unique value that is a statistically unique value for each device.

**Table 3 PSID C_PIN**

| UID | Name | CommonName | PIN | CharSet | TryLimit | Tries | Persistence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 0B   <br> 00 01 FF 01 | "C_PIN_PSID" | “PhysicalDriveOwner” | VU | Null | VU | VU | FALSE |

#### 4.2.2.1 PSID C_PIN Object Access Control

Access control on methods applicable to the PSID C_PIN object are as defined in this section.

**Table 4 PSID C_PIN Access Control Settings**

| Table association - Informative text | UID | InvokingID | InvokingID Name - Iinformative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethod ACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C_PIN |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 01 FF | C_PIN_PSID | G et |  | ACE_C_PIN_Get_PSID_NoPIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |

### 4.2.3 New ACEs

This section defines new ACEs to be added in conjunction with the PSID authority/credential.

#### 4.2.3.1 ACE_C_PIN_Get_PSID_NoPIN

This section defines the ACE that enables retrieval of the attributes of C_PIN_PSID.

Support for this ACE is Mandatory (M) if the PSID authority is supported.

**Table 5 ACE_C_PIN_Get_PSID_NoPIN ACE**

| Table Assocuiation - Informative text | UID | Name | CommonName | BooleanExpr | Columns |
| --- | --- | --- | --- | --- | --- |
| ACE |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 01 00 E1 | “ACE_C_PIN_Get_PSID_NoPIN” |  | Anybody | UID,   <br> CharSet, TryLimit, Tries,  <br> Persistence |

##### 4.2.3.1.1 ACE_C_PIN_Get_PSID_NoPIN Access Control

Get access to ACE objects is as defined by [3].

#### 4.2.3.2 ACE_SP_PSID

This section defines the ACE that allows the PSID authority to be used to perform a device reversion operation. Support for this ACE is Mandatory (M) if the PSID authority is supported.

**Table 6 ACE_SP_PSID ACE**

| Table Assocuiation - Informative text | UID | Name | CommonName | BooleanExpr | Columns |
| --- | --- | --- | --- | --- | --- |
| ACE |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 01 00 E0 | “ACE_SP_PSID” |  | PSID | All |

##### 4.2.3.2.1 ACE_SP_PSID Access Control

Get access to ACE objects is as defined by [3].

### 4.2.4 Performing Device Reversion

This section details the access control settings for using the PSID authority to invoke the Revert method. Support for this capability is Mandatory (M) if the PSID authority is supported.

**Table 7 Device Reversion Access Control Settings**

| Table Association - informative oly | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ACE |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 02 05 00 01 00 00 | AdminSPObj | Revert |  | ACE_SP_SID, ACE_SP_PSID |  |  |  | ACE_Anybody |  |  |  |  |  |  |

## 4.3 Locking SP

This feature set requires no additions to the Locking SP.

## 4.4 Additional SPs

This feature set requires no additional SPs.
