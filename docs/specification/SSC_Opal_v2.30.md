## Table of Contents

- [1 DISCLAIMERS, NOTICES, AND LICENSE TERMS](#1-disclaimers-notices-and-license-terms)
- [2 ACKNOWLEDGEMENT](#2-acknowledgement)
- [3 List of Tables](#3-list-of-tables)
- [4 List of Figures](#4-list-of-figures)
- [1 Introduction](#1-introduction)
  - [1.1 Document Purpose](#11-document-purpose)
  - [1.2 Scope and Intended Audience](#12-scope-and-intended-audience)
  - [1.3 Conventions](#13-conventions)
    - [1.3.1 Key Words](#131-key-words)
    - [1.3.2 Font Conventions](#132-font-conventions)
    - [1.3.3 Statement Types](#133-statement-types)
    - [1.3.4 Cell Shading and Font Legend for Preconfiguration Tables](#134-cell-shading-and-font-legend-for-preconfiguration-tables)
    - [1.3.5 List Conventions](#135-list-conventions)
      - [1.3.5.1 Lists Overview](#1351-lists-overview)
      - [1.3.5.2 Unordered Lists](#1352-unordered-lists)
      - [1.3.5.3 Ordered Lists](#1353-ordered-lists)
    - [1.3.6 Numbering Conventions](#136-numbering-conventions)
    - [1.3.7 Bit Conventions](#137-bit-conventions)
    - [1.3.8 Number Range Conventions](#138-number-range-conventions)
    - [1.3.9 Specify and Indicate Conventions](#139-specify-and-indicate-conventions)
  - [1.4 Document References](#14-document-references)
    - [1.4.1 Document Precedence](#141-document-precedence)
    - [1.4.2 Approved References](#142-approved-references)
    - [1.4.3 References Under Development](#143-references-under-development)
  - [1.5 Dependencies on Other Feature Sets](#15-dependencies-on-other-feature-sets)
  - [1.6 Interactions with Other Feature Sets](#16-interactions-with-other-feature-sets)
  - [1.7 Definition of Terms](#17-definition-of-terms)
- [2 Opal SSC Overview](#2-opal-ssc-overview)
  - [2.1 Opal SSC Use Cases and Threats](#21-opal-ssc-use-cases-and-threats)
  - [2.2 Security Providers (SPs)](#22-security-providers-sps)
  - [2.3 Interface Communication Protocol](#23-interface-communication-protocol)
  - [2.4 Cryptographic Features](#24-cryptographic-features)
  - [2.5 Authentication](#25-authentication)
  - [2.6 Table Management](#26-table-management)
  - [2.7 Access Control & Personalization](#27-access-control-personalization)
  - [2.8 Issuance](#28-issuance)
  - [2.9 SSC Discovery](#29-ssc-discovery)
  - [2.10 Mandatory Feature Sets](#210-mandatory-feature-sets)
- [3 Opal SSC Features](#3-opal-ssc-features)
  - [3.1 Security Protocol 1 Support](#31-security-protocol-1-support)
    - [3.1.1 Level 0 Discovery (M)](#311-level-0-discovery-m)
      - [3.1.1.1 Level 0 Discovery Header](#3111-level-0-discovery-header)
      - [3.1.1.2 TPer Feature (Feature Code = 0x0001)](#3112-tper-feature-feature-code-0x0001)
      - [3.1.1.3 Locking Feature (Feature Code = 0x0002)](#3113-locking-feature-feature-code-0x0002)
        - [3.1.1.3.1 LockingEnabled Definition](#31131-lockingenabled-definition)
      - [3.1.1.4 Geometry Reporting Feature (Feature Code = 0x0003)](#3114-geometry-reporting-feature-feature-code-0x0003)
        - [3.1.1.4.1 Overview](#31141-overview)
        - [3.1.1.4.2 ALIGN](#31142-align)
        - [3.1.1.4.3 LogicalBlockSize](#31143-logicalblocksize)
        - [3.1.1.4.4 AlignmentGranularity](#31144-alignmentgranularity)
        - [3.1.1.4.5 LowestAlignedLBA](#31145-lowestalignedlba)
      - [3.1.1.5 Opal SSC V2 Feature (Feature Code = 0x0203)](#3115-opal-ssc-v2-feature-feature-code-0x0203)
        - [3.1.1.5.1 Base ComID](#31151-base-comid)
        - [3.1.1.5.2 Number of ComIDs](#31152-number-of-comids)
      - [3.1.1.6 Supported Data Removal Mechanism Feature (Feature Code = 0x0404)](#3116-supported-data-removal-mechanism-feature-feature-code-0x0404)
        - [3.1.1.6.1 Data Removal Operation Processing Definition](#31161-data-removal-operation-processing-definition)
        - [3.1.1.6.2 Data Removal Operation Interrupted](#31162-data-removal-operation-interrupted)
        - [3.1.1.6.3 Supported Data Removal Mechanism Definition](#31163-supported-data-removal-mechanism-definition)
        - [3.1.1.6.4 Data Removal Time Format and Data Removal Time Definition](#31164-data-removal-time-format-and-data-removal-time-definition)
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
- [4 Opal SSC-compliant Functions and SPs](#4-opal-ssc-compliant-functions-and-sps)
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
    - [4.2.5 Opal Additional Column Types](#425-opal-additional-column-types)
      - [4.2.5.1 Data_removal_mechanism](#4251-data_removal_mechanism)
    - [4.2.6 Opal Additional Data Structures](#426-opal-additional-data-structures)
      - [4.2.6.1 DataRemovalMechanism (ObjectTable)](#4261-dataremovalmechanism-objecttable)
        - [4.2.6.1.1 UID](#42611-uid)
        - [4.2.6.1.2 ActiveDataRemovalMechanism](#42612-activedataremovalmechanism)
    - [4.2.7 Opal Additional Tables](#427-opal-additional-tables)
      - [4.2.7.1 DataRemovalMechansim (M)](#4271-dataremovalmechansim-m)
    - [4.2.8 Crypto Template Tables](#428-crypto-template-tables)
    - [4.2.9 Crypto Template Methods](#429-crypto-template-methods)
      - [4.2.9.1 Random](#4291-random)
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
        - [4.3.5.2.1 Geometry Reporting Feature Behavior](#43521-geometry-reporting-feature-behavior)
        - [4.3.5.2.2 LockOnReset Restrictions](#43522-lockonreset-restrictions)
      - [4.3.5.3 MBRControl (M)](#4353-mbrcontrol-m)
        - [4.3.5.3.1 DoneOnReset Restrictions](#43531-doneonreset-restrictions)
      - [4.3.5.4 MBR (M)](#4354-mbr-m)
      - [4.3.5.5 K_AES_128 or K_AES_256 (M)](#4355-k_aes_128-or-k_aes_256-m)
    - [4.3.6 Locking Template Methods](#436-locking-template-methods)
    - [4.3.7 Storage Device Read/Write Data Command Locking Behavior Interactions with Range Crossing](#437-storage-device-readwrite-data-command-locking-behavior-interactions-with-range-crossing)
    - [4.3.8 Non Template Tables](#438-non-template-tables)
      - [4.3.8.1 DataStore (M)](#4381-datastore-m)
- [5 Appendix – SSC Specific Features](#5-appendix-ssc-specific-features)
  - [5.1 Opal SSC-Specific Methods](#51-opal-ssc-specific-methods)
    - [5.1.1 Activate – Admin Template SP Object Method](#511-activate-admin-template-sp-object-method)
      - [5.1.1.1 Activate Support](#5111-activate-support)
      - [5.1.1.2 Side effects of Activate](#5112-side-effects-of-activate)
    - [5.1.2 Revert – Admin Template SP Object Method](#512-revert-admin-template-sp-object-method)
      - [5.1.2.1 Revert Support](#5121-revert-support)
      - [5.1.2.2 Effects of Revert](#5122-effects-of-revert)
        - [5.1.2.2.1 Effects of Revert on the PIN Column Value of C_PIN_SID](#51221-effects-of-revert-on-the-pin-column-value-of-c_pin_sid)
      - [5.1.2.3 Interrupted Revert](#5123-interrupted-revert)
    - [5.1.3 RevertSP – Base Template SP Method](#513-revertsp-base-template-sp-method)
      - [5.1.3.1 RevertSP Support](#5131-revertsp-support)
      - [5.1.3.2 KeepGlobalRangeKey parameter (Locking Template-specific)](#5132-keepglobalrangekey-parameter-locking-template-specific)
      - [5.1.3.3 Effects of RevertSP](#5133-effects-of-revertsp)
      - [5.1.3.4 Interrupted RevertSP](#5134-interrupted-revertsp)
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
  - [5.4 Examples of Alignment Geometry Reporting](#54-examples-of-alignment-geometry-reporting)

# 1 DISCLAIMERS, NOTICES, AND LICENSE TERMS

THIS SPECIFICATION IS PROVIDED “AS IS” WITH NO WARRANTIES WHATSOEVER, INCLUDING ANY WARRANTY OF MERCHANTABILITY, NONINFRINGEMENT, FITNESS FOR ANY PARTICULAR PURPOSE, OR ANY WARRANTY OTHERWISE ARISING OUT OF ANY PROPOSAL, SPECIFICATION OR SAMPLE.

Without limitation, TCG disclaims all liability, including liability for infringement of any proprietary rights, relating to use of information in this specification and to the implementation of this specification, and TCG disclaims all liability for cost of procurement of substitute goods or services, lost profits, loss of use, loss of data or any incidental, consequential, direct, indirect, or special damages, whether under contract, tort, warranty or otherwise, arising in any way out of use or reliance upon this specification or any information herein.

This document is copyrighted by Trusted Computing Group (TCG), and no license, express or implied, is granted herein other than as follows: You may not copy or reproduce the document or distribute it to others without written permission from TCG, except that you may freely do so for the purposes of (a) examining or implementing TCG specifications or (b) developing, testing, or promoting information technology standards and best practices, so long as you distribute the document with these disclaimers, notices, and license terms.

Contact the Trusted Computing Group at www.trustedcomputinggroup.org for information on specification licensing through membership agreements.

Any marks and brands contained herein are the property of their respective owners.

# 2 ACKNOWLEDGEMENT

The TCG wishes to thank all those who contributed to this specification. This document builds on work done in various working groups in the TCG and the industry at large.

| NAME | COMPANY |
| --- | --- |
| Chandra Nelogal | Dell, Inc. |
| Joy Shukla | Google Inc. |
| Glen Jaquette | IBM |
| Joerg Borchert | Infineon Technologies |
| Frederick Knight | Kioxia Corporation |
| James Borden | Kioxia Corporation |
| Paul Suhler | Kioxia Corporation |
| Taku Kato | Kioxia Corporation |
| Alan Arnold | Lenovo Inc. |
| Ghassan Tchelepi | Marvell Technology |
| Peter Dinh | Marvell Semiconductor, Inc. |
| Alon Cohen | Microchip Technology, Inc |
| Artem Zankovich | Micron Technology, Inc |
| Bharath Madanayakanahalli Gururaj | Micron Technology, Inc |
| Robert Strong | Micron Technology, Inc |
| Walt Hubis | Micron Technology, Inc |
| Sridhar Balasubramanian | NetApp |
| Tim Chevalier | NetApp |
| Eric Hibbard | Samsung Semiconductor Inc. |
| Anthony Duran | Seagate Technology |
| Saheb Biswas | Seagate Technology |
| Festus Hategekimana | Solidigm |
| John Mathews | Solidigm |
| Patrick Hery | Toshiba Corporation |
| Joseph Chen | ULINK Technology Inc. |
| Yoni Shternhell | Western Digital Technologies, Inc. |
| Jim Hatfield | Invited Expert |
| Michael McDonnell | Invited Expert |

**CONTENTS**

[DISCLAIMERS, NOTICES, AND LICENSE TERMS [1](#disclaimers-notices-and-license-terms)](#disclaimers-notices-and-license-terms)

[ACKNOWLEDGEMENT [2](#acknowledgement)](#acknowledgement)

[LIST OF TABLES [8](#list-of-tables)](#list-of-tables)

[LIST OF FIGURES [9](#list-of-figures)](#list-of-figures)

[1 INTRODUCTION [10](#introduction)](#introduction)

[1.1 Document Purpose [10](#document-purpose)](#document-purpose)

[1.2 Scope and Intended Audience [10](#scope-and-intended-audience)](#scope-and-intended-audience)

[1.3 Conventions [10](#conventions)](#conventions)

[1.3.1 Key Words [10](#key-words)](#key-words)

[1.3.2 Font Conventions [10](#font-conventions)](#font-conventions)

[1.3.3 Statement Types [10](#statement-types)](#statement-types)

[1.3.4 Cell Shading and Font Legend for Preconfiguration Tables [11](#cell-shading-and-font-legend-for-preconfiguration-tables)](#cell-shading-and-font-legend-for-preconfiguration-tables)

[1.3.5 List Conventions [12](#list-conventions)](#list-conventions)

[1.3.5.1 Lists Overview [12](#lists-overview)](#lists-overview)

[1.3.5.2 Unordered Lists [12](#unordered-lists)](#unordered-lists)

[1.3.5.3 Ordered Lists [12](#ordered-lists)](#ordered-lists)

[1.3.6 Numbering Conventions [12](#numbering-conventions)](#numbering-conventions)

[1.3.7 Bit Conventions [13](#bit-conventions)](#bit-conventions)

[1.3.8 Number Range Conventions [13](#number-range-conventions)](#number-range-conventions)

[1.3.9 Specify and Indicate Conventions [13](#specify-and-indicate-conventions)](#specify-and-indicate-conventions)

[1.4 Document References [13](#document-references)](#document-references)

[1.4.1 Document Precedence [13](#document-precedence)](#document-precedence)

[1.4.2 Approved References [13](#approved-references)](#approved-references)

[1.4.3 References Under Development [14](#references-under-development)](#references-under-development)

[1.5 Dependencies on Other Feature Sets [14](#dependencies-on-other-feature-sets)](#dependencies-on-other-feature-sets)

[1.6 Interactions with Other Feature Sets [14](#interactions-with-other-feature-sets)](#interactions-with-other-feature-sets)

[1.7 Definition of Terms [14](#definition-of-terms)](#definition-of-terms)

[2 OPAL SSC OVERVIEW [16](#opal-ssc-overview)](#opal-ssc-overview)

[2.1 Opal SSC Use Cases and Threats [16](#opal-ssc-use-cases-and-threats)](#opal-ssc-use-cases-and-threats)

[2.2 Security Providers (SPs) [16](#security-providers-sps)](#security-providers-sps)

[2.3 Interface Communication Protocol [16](#interface-communication-protocol)](#interface-communication-protocol)

[2.4 Cryptographic Features [16](#cryptographic-features)](#cryptographic-features)

[2.5 Authentication [16](#authentication)](#authentication)

[2.6 Table Management [17](#table-management)](#table-management)

[2.7 Access Control & Personalization [17](#access-control-personalization)](#access-control-personalization)

[2.8 Issuance [17](#issuance)](#issuance)

[2.9 SSC Discovery [17](#ssc-discovery)](#ssc-discovery)

[2.10 Mandatory Feature Sets [17](#mandatory-feature-sets)](#mandatory-feature-sets)

[3 OPAL SSC FEATURES [18](#opal-ssc-features)](#opal-ssc-features)

[3.1 Security Protocol 1 Support [18](#security-protocol-1-support)](#security-protocol-1-support)

[3.1.1 Level 0 Discovery (M) [18](#level-0-discovery-m)](#level-0-discovery-m)

[3.1.1.1 Level 0 Discovery Header [18](#level-0-discovery-header)](#level-0-discovery-header)

[3.1.1.2 TPer Feature (Feature Code = 0x0001) [19](#tper-feature-feature-code-0x0001)](#tper-feature-feature-code-0x0001)

[3.1.1.3 Locking Feature (Feature Code = 0x0002) [19](#locking-feature-feature-code-0x0002)](#locking-feature-feature-code-0x0002)

[3.1.1.3.1 LockingEnabled Definition [20](#lockingenabled-definition)](#lockingenabled-definition)

[3.1.1.4 Geometry Reporting Feature (Feature Code = 0x0003) [20](#geometry-reporting-feature-feature-code-0x0003)](#geometry-reporting-feature-feature-code-0x0003)

[3.1.1.4.1 Overview [20](#overview)](#overview)

[3.1.1.4.2 ALIGN [21](#align)](#align)

[3.1.1.4.3 LogicalBlockSize [21](#logicalblocksize)](#logicalblocksize)

[3.1.1.4.4 AlignmentGranularity [21](#alignmentgranularity)](#alignmentgranularity)

[3.1.1.4.5 LowestAlignedLBA [21](#lowestalignedlba)](#lowestalignedlba)

[3.1.1.5 Opal SSC V2 Feature (Feature Code = 0x0203) [22](#opal-ssc-v2-feature-feature-code-0x0203)](#opal-ssc-v2-feature-feature-code-0x0203)

[3.1.1.5.1 Base ComID [23](#base-comid)](#base-comid)

[3.1.1.5.2 Number of ComIDs [23](#number-of-comids)](#number-of-comids)

[3.1.1.6 Supported Data Removal Mechanism Feature (Feature Code = 0x0404) [23](#supported-data-removal-mechanism-feature-feature-code-0x0404)](#supported-data-removal-mechanism-feature-feature-code-0x0404)

[3.1.1.6.1 Data Removal Operation Processing Definition [25](#data-removal-operation-processing-definition)](#data-removal-operation-processing-definition)

[3.1.1.6.2 Data Removal Operation Interrupted [25](#data-removal-operation-interrupted)](#data-removal-operation-interrupted)

[3.1.1.6.3 Supported Data Removal Mechanism Definition [26](#supported-data-removal-mechanism-definition)](#supported-data-removal-mechanism-definition)

[3.1.1.6.4 Data Removal Time Format and Data Removal Time Definition [26](#data-removal-time-format-and-data-removal-time-definition)](#data-removal-time-format-and-data-removal-time-definition)

[3.2 Security Protocol 2 Support [27](#security-protocol-2-support)](#security-protocol-2-support)

[3.2.1 ComID Management [27](#comid-management)](#comid-management)

[3.2.2 Stack Protocol Reset (M) [27](#stack-protocol-reset-m)](#stack-protocol-reset-m)

[3.2.3 TPER_RESET command (M) [27](#tper_reset-command-m)](#tper_reset-command-m)

[3.3 Communications [28](#communications)](#communications)

[3.3.1 Communication Properties [28](#communication-properties)](#communication-properties)

[3.3.2 Supported Security Protocols [28](#supported-security-protocols)](#supported-security-protocols)

[3.3.3 ComIDs [29](#comids)](#comids)

[3.3.4 Synchronous Protocol [29](#synchronous-protocol)](#synchronous-protocol)

[3.3.4.1 Payload Encoding [29](#payload-encoding)](#payload-encoding)

[3.3.4.1.1 Stream Encoding Modifications [29](#stream-encoding-modifications)](#stream-encoding-modifications)

[3.3.4.1.2 TCG Packets [30](#tcg-packets)](#tcg-packets)

[3.3.4.1.3 Payload Error Response [30](#payload-error-response)](#payload-error-response)

[3.3.5 Storage Device Resets [30](#storage-device-resets)](#storage-device-resets)

[3.3.5.1 Interface Resets [30](#interface-resets)](#interface-resets)

[3.3.5.2 TCG Reset Events [31](#tcg-reset-events)](#tcg-reset-events)

[3.3.6 Protocol Stack Reset Commands (M) [31](#protocol-stack-reset-commands-m)](#protocol-stack-reset-commands-m)

[4 OPAL SSC-COMPLIANT FUNCTIONS AND SPS [32](#opal-ssc-compliant-functions-and-sps)](#opal-ssc-compliant-functions-and-sps)

[4.1 Session Manager [32](#session-manager)](#session-manager)

[4.1.1 Methods [32](#methods)](#methods)

[4.1.1.1 Properties (M) [32](#properties-m)](#properties-m)

[4.1.1.2 StartSession (M) [33](#startsession-m)](#startsession-m)

[4.1.1.3 SyncSession (M) [34](#syncsession-m)](#syncsession-m)

[4.1.1.4 CloseSession (O) [34](#closesession-o)](#closesession-o)

[4.2 Admin SP [34](#admin-sp)](#admin-sp)

[4.2.1 Base Template Tables [34](#base-template-tables)](#base-template-tables)

[4.2.1.1 SPInfo (M) [34](#spinfo-m)](#spinfo-m)

[4.2.1.2 SPTemplates (M) [34](#sptemplates-m)](#sptemplates-m)

[4.2.1.3 Table (M) [35](#table-m)](#table-m)

[4.2.1.4 MethodID (M) [37](#methodid-m)](#methodid-m)

[4.2.1.5 AccessControl (M) [37](#accesscontrol-m)](#accesscontrol-m)

[4.2.1.6 ACE (M) [45](#ace-m)](#ace-m)

[4.2.1.7 Authority (M) [46](#authority-m)](#authority-m)

[4.2.1.8 C_PIN (M) [47](#c_pin-m)](#c_pin-m)

[4.2.2 Base Template Methods [48](#base-template-methods)](#base-template-methods)

[4.2.3 Admin Template Tables [48](#admin-template-tables)](#admin-template-tables)

[4.2.3.1 TPerInfo (M) [48](#tperinfo-m)](#tperinfo-m)

[4.2.3.2 Template (M) [49](#template-m)](#template-m)

[4.2.3.3 SP (M) [49](#sp-m)](#sp-m)

[4.2.4 Admin Template Methods [50](#admin-template-methods)](#admin-template-methods)

[4.2.5 Opal Additional Column Types [50](#opal-additional-column-types)](#opal-additional-column-types)

[4.2.5.1 Data_removal_mechanism [50](#data_removal_mechanism)](#data_removal_mechanism)

[4.2.6 Opal Additional Data Structures [50](#opal-additional-data-structures)](#opal-additional-data-structures)

[4.2.6.1 DataRemovalMechanism (ObjectTable) [50](#dataremovalmechanism-objecttable)](#dataremovalmechanism-objecttable)

[4.2.6.1.1 UID [50](#uid)](#uid)

[4.2.6.1.2 ActiveDataRemovalMechanism [50](#activedataremovalmechanism)](#activedataremovalmechanism)

[4.2.7 Opal Additional Tables [51](#opal-additional-tables)](#opal-additional-tables)

[4.2.7.1 DataRemovalMechansim (M) [51](#dataremovalmechansim-m)](#dataremovalmechansim-m)

[4.2.8 Crypto Template Tables [51](#crypto-template-tables)](#crypto-template-tables)

[4.2.9 Crypto Template Methods [51](#crypto-template-methods)](#crypto-template-methods)

[4.2.9.1 Random [51](#random)](#random)

[4.3 Locking SP [51](#locking-sp)](#locking-sp)

[4.3.1 Base Template Tables [51](#base-template-tables-1)](#base-template-tables-1)

[4.3.1.1 SPInfo (M) [51](#spinfo-m-1)](#spinfo-m-1)

[4.3.1.2 SPTemplates (M) [51](#sptemplates-m-1)](#sptemplates-m-1)

[4.3.1.3 Table (M) [52](#table-m-1)](#table-m-1)

[4.3.1.4 Type (N) [53](#type-n)](#type-n)

[4.3.1.5 MethodID (M) [53](#methodid-m-1)](#methodid-m-1)

[4.3.1.6 AccessControl (M) [54](#accesscontrol-m-1)](#accesscontrol-m-1)

[4.3.1.7 ACE (M) [77](#ace-m-1)](#ace-m-1)

[4.3.1.8 Authority (M) [81](#authority-m-1)](#authority-m-1)

[4.3.1.9 C_PIN (M) [82](#c_pin-m-1)](#c_pin-m-1)

[4.3.1.10 SecretProtect (M) [83](#secretprotect-m)](#secretprotect-m)

[4.3.2 Base Template Methods [83](#base-template-methods-1)](#base-template-methods-1)

[4.3.3 Crypto Template Tables [84](#crypto-template-tables-1)](#crypto-template-tables-1)

[4.3.4 Crypto Template Methods [84](#crypto-template-methods-1)](#crypto-template-methods-1)

[4.3.4.1 Random [84](#random-1)](#random-1)

[4.3.5 Locking Template Tables [84](#locking-template-tables)](#locking-template-tables)

[4.3.5.1 LockingInfo (M) [84](#lockinginfo-m)](#lockinginfo-m)

[4.3.5.2 Locking (M) [85](#locking-m)](#locking-m)

[4.3.5.2.1 Geometry Reporting Feature Behavior [86](#geometry-reporting-feature-behavior)](#geometry-reporting-feature-behavior)

[4.3.5.2.2 LockOnReset Restrictions [87](#lockonreset-restrictions)](#lockonreset-restrictions)

[4.3.5.3 MBRControl (M) [87](#mbrcontrol-m)](#mbrcontrol-m)

[4.3.5.3.1 DoneOnReset Restrictions [87](#doneonreset-restrictions)](#doneonreset-restrictions)

[4.3.5.4 MBR (M) [88](#mbr-m)](#mbr-m)

[4.3.5.5 K_AES_128 or K_AES_256 (M) [88](#k_aes_128-or-k_aes_256-m)](#k_aes_128-or-k_aes_256-m)

[4.3.6 Locking Template Methods [89](#locking-template-methods)](#locking-template-methods)

[4.3.7 Storage Device Read/Write Data Command Locking Behavior Interactions with Range Crossing [89](#storage-device-readwrite-data-command-locking-behavior-interactions-with-range-crossing)](#storage-device-readwrite-data-command-locking-behavior-interactions-with-range-crossing)

[4.3.8 Non Template Tables [89](#non-template-tables)](#non-template-tables)

[4.3.8.1 DataStore (M) [89](#datastore-m)](#datastore-m)

[5 APPENDIX – SSC SPECIFIC FEATURES [90](#appendix-ssc-specific-features)](#appendix-ssc-specific-features)

[5.1 Opal SSC-Specific Methods [90](#opal-ssc-specific-methods)](#opal-ssc-specific-methods)

[5.1.1 Activate – Admin Template SP Object Method [90](#activate-admin-template-sp-object-method)](#activate-admin-template-sp-object-method)

[5.1.1.1 Activate Support [90](#activate-support)](#activate-support)

[5.1.1.2 Side effects of Activate [90](#side-effects-of-activate)](#side-effects-of-activate)

[5.1.2 Revert – Admin Template SP Object Method [90](#revert-admin-template-sp-object-method)](#revert-admin-template-sp-object-method)

[5.1.2.1 Revert Support [91](#revert-support)](#revert-support)

[5.1.2.2 Effects of Revert [91](#effects-of-revert)](#effects-of-revert)

[5.1.2.2.1 Effects of Revert on the PIN Column Value of C_PIN_SID [92](#effects-of-revert-on-the-pin-column-value-of-c_pin_sid)](#effects-of-revert-on-the-pin-column-value-of-c_pin_sid)

[5.1.2.3 Interrupted Revert [92](#interrupted-revert)](#interrupted-revert)

[5.1.3 RevertSP – Base Template SP Method [93](#revertsp-base-template-sp-method)](#revertsp-base-template-sp-method)

[5.1.3.1 RevertSP Support [93](#revertsp-support)](#revertsp-support)

[5.1.3.2 KeepGlobalRangeKey parameter (Locking Template-specific) [93](#keepglobalrangekey-parameter-locking-template-specific)](#keepglobalrangekey-parameter-locking-template-specific)

[5.1.3.3 Effects of RevertSP [93](#effects-of-revertsp)](#effects-of-revertsp)

[5.1.3.4 Interrupted RevertSP [94](#interrupted-revertsp)](#interrupted-revertsp)

[5.2 Life Cycle [94](#life-cycle)](#life-cycle)

[5.2.1 Issued vs. Manufactured SPs [94](#issued-vs.-manufactured-sps)](#issued-vs.-manufactured-sps)

[5.2.1.1 Issued SPs [94](#issued-sps)](#issued-sps)

[5.2.1.2 Manufactured SPs [94](#manufactured-sps)](#manufactured-sps)

[5.2.2 Manufactured SP Life Cycle States [95](#manufactured-sp-life-cycle-states)](#manufactured-sp-life-cycle-states)

[5.2.2.1 State definitions for Manufactured SPs [95](#state-definitions-for-manufactured-sps)](#state-definitions-for-manufactured-sps)

[5.2.2.2 State transitions for Manufactured SPs [96](#state-transitions-for-manufactured-sps)](#state-transitions-for-manufactured-sps)

[5.2.2.2.1 Manufactured-Inactive to Manufactured [96](#manufactured-inactive-to-manufactured)](#manufactured-inactive-to-manufactured)

[5.2.2.2.2 ANY STATE to ORIGINAL FACTORY STATE [96](#any-state-to-original-factory-state)](#any-state-to-original-factory-state)

[5.2.2.3 State behaviors for Manufactured SPs [96](#state-behaviors-for-manufactured-sps)](#state-behaviors-for-manufactured-sps)

[5.2.2.3.1 Manufactured-Inactive [96](#manufactured-inactive)](#manufactured-inactive)

[5.2.2.3.2 Manufactured [97](#manufactured)](#manufactured)

[5.2.3 Type Table Modification [97](#type-table-modification)](#type-table-modification)

[5.3 Byte Table Access Granularity [97](#byte-table-access-granularity)](#byte-table-access-granularity)

[5.3.1 Table Table Modification [97](#table-table-modification)](#table-table-modification)

[5.3.1.1 MandatoryWriteGranularity [98](#mandatorywritegranularity)](#mandatorywritegranularity)

[5.3.1.1.1 Object Tables [98](#object-tables)](#object-tables)

[5.3.1.1.2 Byte Tables [98](#byte-tables)](#byte-tables)

[5.3.1.2 RecommendedAccessGranularity [98](#recommendedaccessgranularity)](#recommendedaccessgranularity)

[5.3.1.2.1 Object Tables [98](#object-tables-1)](#object-tables-1)

[5.3.1.2.2 Byte Tables [99](#byte-tables-1)](#byte-tables-1)

[5.4 Examples of Alignment Geometry Reporting [99](#examples-of-alignment-geometry-reporting)](#examples-of-alignment-geometry-reporting)

# 3 List of Tables

Table 1 - Preconfiguration Tables Legend ................................................................................................................. 11

Table 2 - Level 0 Discovery Header .......................................................................................................................... 18 Table 3 - Level 0 Discovery - TPer Feature Descriptor .............................................................................................. 19 Table 4 - Level 0 Discovery - Locking Feature Descriptor ......................................................................................... 19 Table 5 - Level 0 Discovery - Geometry Reporting Feature Descriptor ..................................................................... 20 Table 6 - Level 0 Discovery - Opal SSC V2 Feature Descriptor ................................................................................ 22 Table 7 - SSC Minor Versions ................................................................................................................................... 23 Table 8 - Level 0 Discovery – Supported Data Removal Mechanism Feature Descriptor ......................................... 24

Table 9 - Parameter explanation ............................................................................................................................... 25 Table 10 - Supported Data Removal Mechanism ...................................................................................................... 26 Table 11 - Data Removal Time (Data Removal Time Format bit= 0) ......................................................................... 27

Table 12 - Data Removal Time (Data Removal Time Format bit= 1) ......................................................................... 27 Table 13 - TPER_RESET Command ........................................................................................................................ 28 Table 14 - ComID Assignments ................................................................................................................................. 29 Table 15 - Supported Tokens .................................................................................................................................... 29 Table 16 - reset_types ............................................................................................................................................... 31 Table 17 - Properties Requirements .......................................................................................................................... 32 Table 18 - Admin SP - SPInfo Table Preconfiguration .............................................................................................. 34 Table 19 - Admin SP - SPTemplates Table Preconfiguration .................................................................................... 34 Table 20 - Admin SP - Table Table Preconfiguration ................................................................................................ 35 Table 21 - Admin SP - MethodID Table Preconfiguration .......................................................................................... 37 Table 22 - Admin SP - AccessControl Table Preconfiguration .................................................................................. 38 Table 23 - Admin SP - ACE Table Preconfiguration .................................................................................................. 45 Table 24 - Admin SP - Authority Table Preconfiguration ........................................................................................... 47

Table 25 - Admin SP - C_PIN Table Preconfiguration ............................................................................................... 47 Table 26 - Admin SP – TPerInfo Columns ................................................................................................................. 48 Table 27 - Admin SP - TPerInfo Table Preconfiguration............................................................................................ 49 Table 28 - Admin SP - Template Table Preconfiguration .......................................................................................... 49 Table 29 - Admin SP - SP Table Preconfiguration .................................................................................................... 49 Table 30 - data_removal_mechanism Type Table Addition ....................................................................................... 50 Table 31 - data_removal_mechanism Enumeration Values ...................................................................................... 50 Table 32 - DataRemovalMechansim Table Description ............................................................................................. 50 Table 33 - Admin SP – DataRemovalMechansim Table Preconfiguration ................................................................. 51 Table 34 - Locking SP - SPInfo Table Preconfiguration ............................................................................................ 51 Table 35 - Locking SP - SPTemplates Table Preconfiguration .................................................................................. 52 Table 36 - Locking SP - Table Table Preconfiguration .............................................................................................. 52 Table 37 - Locking SP - MethodID Table Preconfiguration ........................................................................................ 54 Table 38 - Locking SP - AccessControl Table Preconfiguration ................................................................................ 55 Table 39 - Locking SP - ACE Table Preconfiguration ................................................................................................ 77 Table 40 - Locking SP - Authority Table Preconfiguration ......................................................................................... 81

Table 41 - Locking SP - C_PIN Table Preconfiguration ............................................................................................. 83 Table 42 - Locking SP - SecretProtect Table Preconfiguration ................................................................................. 83 Table 43 - Locking SP – LockingInfo Columns .......................................................................................................... 84 Table 44 - Locking SP - LockingInfo Table Preconfiguration ..................................................................................... 84 Table 45 - Locking SP - Locking Table Preconfiguration ........................................................................................... 85 Table 46 - Locking SP - MBRControl Table Preconfiguration .................................................................................... 87 Table 47 - Locking SP - K_AES_128 Table Preconfiguration .................................................................................... 88

Table 48 - Locking SP - K_AES_256 Table Preconfiguration .................................................................................... 88 Table 49 - Life Cycle State Type Table Modification ................................................................................................. 97 Table 50 - Table Table Additional Columns ............................................................................................................... 97

# 4 List of Figures

Figure 1 - StartAlignment Calculation ........................................................................................................................ 86

Figure 2 - LengthAlignment Calculation ..................................................................................................................... 87 Figure 3 - Life Cycle State Diagram for Manufactured SPs ....................................................................................... 95 Figure 4 - ValidMandatoryGranularity definition ........................................................................................................ 98 Figure 5 - ValidRecommendedGranularity definition for Set ...................................................................................... 99 Figure 6 - ValidRecommendedGranularity definition for Get ..................................................................................... 99 Figure 7 - Example: AlignmentGranularity=1, Lowest Aligned LBA=0 ..................................................................... 100

Figure 8 - Example: AlignmentGranularity=8, Lowest Aligned LBA=0 ..................................................................... 100

Figure 9 - Example: AlignmentGranularity=8, Lowest Aligned LBA=1 ..................................................................... 100 Figure 10 - Example: AlignmentGranularity=2000, Lowest Aligned LBA=1234 ....................................................... 100

# 1 Introduction

## 1.1 Document Purpose

Storage Workgroup specifications provide a comprehensive architecture for putting Storage Devices under policy control as determined by the trusted platform host, the capabilities of the Storage Device to conform to the policies of the trusted platform, and the lifecycle state of the Storage Device as a Trusted Peripheral.

## 1.2 Scope and Intended Audience

This specification defines the Opal Security Subsystem Class (SSC). Any Storage Device that claims compliance to the Opal SSC SHALL conform to this specification.

The intended audience for this specification is both trusted Storage Device manufacturers and developers that want to use these Storage Devices in their systems.

## 1.3 Conventions

### 1.3.1 Key Words

Key words are used to signify SSC requirements.

The Key Words “SHALL”, “SHALL NOT”, “SHOULD,” and “MAY” are used in this document. These words are a subset of the RFC 2119 (see [1]) key words used by TCG. These key words are to be interpreted as described in [1].

In addition to the above key words, the following are also used in this document to describe the requirements of particular features, including tables, methods, and usages thereof.

- Mandatory (M): When a feature is Mandatory, the feature SHALL be implemented. A Compliance test SHALL validate that the feature is operational.
- Optional (O): When a feature is Optional, the feature MAY be implemented. If implemented, a Compliance test SHALL validate that the feature is operational.
- Excluded (X): When a feature is Excluded, the feature SHALL NOT be implemented. A Compliance test SHALL validate that the feature is not operational.
- Not Required (N): When a feature is Not Required, the feature MAY be implemented. No Compliance test is required.

### 1.3.2 Font Conventions

Names of methods and SP tables are in Courier New font (e.g., the Set method, the Locking table). This convention does not apply to method and table names appearing in headings or captions. Hexadecimal numbers are in Courier New font. All other text is in the Arial font.

### 1.3.3 Statement Types

Please note a very important distinction between different sections of text throughout this document. There are two distinctive kinds of text: informative comment and normative statements.

Because most of the text in this specification will be of the kind normative statements, the authors have informally defined it as the default and, as such, have specifically called out text of the kind informative comment. They have done this by flagging the beginning and end of each informative comment and highlighting its text in gray.

This means that unless text is specifically marked as of the kind informative comment, it can be considered a kind of normative statements.

**EXAMPLE: Start of Informative Comment**

This is the first paragraph of 1–n paragraphs containing text of the kind informative comment ...

This is the second paragraph of text of the kind informative comment ...

This is the nth paragraph of text of the kind informative comment ...

To understand the TCG specification the user must read the specification. (This use of MUST does not require any action).

**End of Informative Comment**

### 1.3.4 Cell Shading and Font Legend for Preconfiguration Tables

The legend in Table 1 defines the Preconfiguration tables cell color coding, with the RGB values for the shading of each cell indicated in parentheses. This color coding is informative only. A Preconfiguration table cell content is normative. If a Preconfiguration table cell is empty or blank (regardless of shading), then the contents of that cell are not specified in this specification. The contents may be specified in another specification.

**Table 1 - Preconfiguration Tables Legend**

| Table Cell Legend | R-W | Value | Access Control |  | Comment |
| --- | --- | --- | --- | --- | --- |
| Arial-Narrow (230, 230, 230) | Read-only | Opal SSC specified | Fixed | •  <br> •  <br> • | Cell content is Read-Only.  <br> Access control is fixed.  <br> Value is specified by the Opal  <br> SSC |
| Arial Narrow boldunder  <br> (230, 230, 230) | Read-only | VU | Fixed | • •  <br> • | Cell content is Read-Only.  <br> Access Control is fixed.  <br> Values are Vendor Unique (VU). A minimum or maximum value may be specified. |
| Arial-Narrow  <br> (0, 0, 0) | Not Defined | (N) | Not Defined | • •  <br> • | Cell content is (N).  <br> Access control is not defined. Any text in table cell is informative only. |
|  |  |  |  | • | A Get MAY omit this column from the method response. |
| Arial Narrow boldunder  <br> (179, 179, 179) | Write | Preconfigured,  user  <br> personalizable | Preconfigured, user  <br> personalizable | •  <br> •  <br> • | Cell content is writable.  <br> Access control is personalizable Get Access Control is not described by this color coding |
| Arial-Narrow (179, 179, 179) | Write | Preconfigured, user  <br> personalizable | Fixed | • •  <br> • | Cell content is writable.  <br> Access control is fixed. Get Access Control is not described by this color coding |

### 1.3.5 List Conventions

#### 1.3.5.1 Lists Overview

Lists are associated with an introductory paragraph or phrase, and are numbered relative to that paragraph or phrase (i.e., all lists begin with an a) or 1) entry).

Each item in a list is preceded by identification with the style of the identification being determined by whether the list is intended to be an ordered list or an unordered list.

If the item in a list is not a complete sentence, the first word in the item is not capitalized. If the item in a list is a complete sentence, the first word in the item is capitalized.

Each item in a list ends with a semicolon, except the last item, which ends in a period. The next to the last entry in the list ends with a semicolon followed by an “and” or an “or” (i.e., “…; and”, or “…; or”). The “and” is used if all the items in the list are required. The “or” is used if only one or more items in the list are required.

#### 1.3.5.2 Unordered Lists

An unordered list is one in which the order of the listed items is unimportant (i.e., it does not matter where in the list an item occurs as all items have equal importance). Each list item shall start with a lowercase letter followed by a close parenthesis. If it is necessary to subdivide a list item further with an additional unordered list (i.e., have a nested unordered list), then the nested unordered list shall be indented and each item in the nested unordered list shall start with an uppercase letter followed by a close parenthesis.

The following is an example of an unordered list with a nested unordered list:

EXAMPLE - The following are the items for the assembly: a) a box containing:

1. a bolt;
2. a nut; and
3. a washer;

2. a screwdriver; and
3. a wrench.

#### 1.3.5.3 Ordered Lists

An ordered list is one in which the order of the listed items is important (i.e., item n is required before item n+1). Each listed item starts with a Western-Arabic numeral followed by a close parenthesis. If it is necessary to subdivide a list item further with an additional unordered list (i.e., have a nested unordered list), then the nested unordered list shall be indented and each item in the nested unordered list shall start with an uppercase letter followed by a close parenthesis.

The following is an example of an ordered list with a nested unordered list:

EXAMPLE - The following are the instructions for the assembly:

1. remove the contents from the box;
2. assemble the item;
  1. use a screwdriver to tighten the screws; and
  2. use a wrench to tighten the bolts; and
3. take a break.

### 1.3.6 Numbering Conventions

A binary number is represented in this specification by any sequence of digits consisting of only the Western-Arabic numerals 0 and 1 immediately followed by a lowercase b (e.g., 0101b). Underscores or spaces may be included between characters in binary number representations to increase readability or delineate field boundaries

(e.g., 0 0101 1010b or 0_0101_1010b).

A hexadecimal number is represented in this specification by any sequence of digits consisting of only the WesternArabic numerals 0 through 9 and/or the uppercase English letters A through F immediately preceded by “0x”. Underscores or spaces may be included between characters in hexadecimal number representations to increase readability or delineate field boundaries (e.g., 0xFD8C FA23 or 0x0B_FD8C_FA23). Hexadecimal numbers are in Courier New font.

A decimal number is represented in this specification by any sequence of digits consisting of only the Western-Arabic numerals 0 through 9 not immediately followed by a lowercase b or lowercase h (e.g., 25). This specification uses the following conventions for representing decimal numbers:

1. the decimal separator (i.e., separating the integer and fractional portions of the number) is a period;
2. the thousands separator (i.e., separating groups of three digits in a portion of the number) is a space; and
3. the thousands separator is used in both the integer portion and the fraction portion of a number.

A decimal number represented in this specification with an overline over one or more digits following the decimal point is a number where the overlined digits are infinitely repeating (e.g., 666.6 **Error! Bookmark not defined.**means 666.666 666… or 666 2/3, and 12.142857 means 12.142 857 142 857… or 12 1/7).

### 1.3.7 Bit Conventions

Name (n:m), where n is greater than m, denotes a set of bits (e.g., Feature (7:0)).

### 1.3.8 Number Range Conventions

p..q, where p is less than q, represents a range of numbers (e.g., words 100..103 represents words 100, 101, 102, and 103).

### 1.3.9 Specify and Indicate Conventions

In a given message, command, or other information exchange between a requestor (e.g., a host) and a responder (e.g., a Storage Device):

1. the word ‘specifies’ means that the requestor provides information in the request; and
2. the word ‘indicates’ means that the responder provides information in the response.

## 1.4 Document References

### 1.4.1 Document Precedence

If there is a conflict between this specification and any other reference, then the precedence is (where a lower number indicates higher precedence):

1. this specification;
2. references under development (see section 1.4.3); and
3. approved references (see section 1.4.2) .

Each reference under development and each approved reference may specify its own document precedence.

### 1.4.2 Approved References

[1]. IETF RFC 2119, 1997, “Key words for use in RFCs to Indicate Requirement Levels”

[2]. Trusted Computing Group (TCG), “TCG Storage Architecture Core Specification”, Version 2.01 [3]. NIST, FIPS-197, 2001, “Advanced Encryption Standard (AES)”

[4]. Trusted Computing Group (TCG), “TCG Storage Interface Interactions Specification“, Version 1.11

[5]. Trusted Computing Group (TCG), “TCG Storage Security Subsystem Class: Opal”, Versions 1.00, 2.00, 2.01,

2.02

[6]. Trusted Computing Group (TCG), “TCG Storage Opal Family Feature Set: Additional DataStore Tables”, Version

1.01

[7]. Trusted Computing Group (TCG), “TCG Storage Opal SSC Feature Set: PSID”, Version 1.00

[8]. Trusted Computing Group (TCG), “TCG Storage Feature Set: Block SID Authentication”, Version 1.01

### 1.4.3 References Under Development

This specification does not have any references under development.

## 1.5 Dependencies on Other Feature Sets

This specification does not depend upon any other feature sets.

## 1.6 Interactions with Other Feature Sets

In the event of conflicting information in this specification and other documents, the precedence for requirements is:

1. this specification;
2. TCG Storage Interface Interactions Specification; and
3. TCG Storage Architecture Core Specification.

## 1.7 Definition of Terms

| Term | Definition |
| --- | --- |
| data removal operation | User Data Removal Method (see [4]) |
| Eradicate | irrevocably erase (e.g., cryptographically erase) |
| IF-RECV | An interface command used to retrieve security protocol data from the TPer (see [4]) |
| IF-SEND | An interface command used to transmit security protocol data to the TPer (see [4]) |
| Manufactured SP | A Manufactured SP is an SP that was created and preconfigured during the Storage Device manufacturing process |
| MM MM | The LSBs of a User Authority object’s UID (hexadecimal) as well as the corresponding C_PIN credential object’s UID  <br> (hexadecimal) |
| NN NN | The LSBs of a Locking object’s UID (hexadecimal) as well as the corresponding K_AES_128/K_AES_256 object’s UID  <br> (hexadecimal) |
| Term | Definition |
| XX XX | The LSBs of an Admin Authority object’s UID (hexadecimal) as well as the corresponding C_PIN credential object’s UID  <br> (hexadecimal) |
| N/A | Not Applicable |
| Original Factory State (OFS) | The original state of an SP when it was created in manufacturing, including its table data, access control settings, and life cycle state.  Each Manufactured SP has its own Original Factory State.  <br> Original Factory State applies to Manufactured SPs only. |
| Preconfiguration Data | The default data in the OFS. |
| SD | Storage Device |
| SP | Security Provider |
| SSC | Security Subsystem Class. SSC specifications describe profiled sets of TCG functionality |
| TPer | Trusted Peripheral |
| Vendor Unique (VU) | These values are unique to each SD manufacturer. Typically VU is used in table cells. |

# 2 Opal SSC Overview

## 2.1 Opal SSC Use Cases and Threats

| Start of Informative Comment  <br> The Opal SSC is an implementation profile for Storage Devices built to:  <br> Protect the confidentiality of stored user data against unauthorized access once it leaves the owner’s control (following a power cycle and subsequent deauthentication) • Enable interoperability between multiple Storage Device vendors An Opal SSC compliant Storage Device:  <br> Facilitates feature discoverability  <br> Provides some user definable features (e.g. access control, locking ranges, user passwords, etc.)  <br> Supports Opal SSC unique behaviors (e.g. communication, table management) This specification addresses a limited set of use cases. They are:  <br> Deploy Storage Device & Take Ownership: the Storage Device is integrated into its target system and ownership transferred by setting or changing the Storage Device’s owner credential.  <br> Activate or Enroll Storage Device: LBA ranges are configured and data encryption and access control credentials (re)generated and/or set on the Storage Device. Access control is configured for LBA range unlocking.  <br> Lock & Unlock Storage Device: unlocking of one or more LBA ranges by the host and locking of those ranges under host control via either an explicit lock or implicit lock triggered by a reset event. MBR shadowing provides a mechanism to boot into a secure pre-boot authentication environment to handle device unlocking.  <br> Repurpose & End-of-Life: erasure of data within one or more LBA ranges and reset of locking credential(s) for Storage Device repurposing or decommissioning.   <br> End of Informative Comment |
| --- |

## 2.2 Security Providers (SPs)

An Opal SSC compliant Storage Device SHALL support at least two Security Providers (SPs):

1. Admin SP
2. Locking SP

The Locking SP MAY be created by the Storage Device manufacturer.

## 2.3 Interface Communication Protocol

An Opal SSC compliant Storage Device SHALL implement the synchronous communications protocol as defined in Section 3.3.4.

This communication protocol operates based upon configuration information defined by:

1) the values reported via Level 0 Discovery (see section 3.1.1);

The combination of the host's communication properties and the TPer's communication properties (see section 4.1.1.1).

## 2.4 Cryptographic Features

An Opal SSC compliant Storage Device SHALL implement Full Disk Encryption for all host accessible user data stored on media. AES-128 or AES-256 SHALL be supported (see [3]).

## 2.5 Authentication

An Opal SSC compliant Storage Device SHALL support password authorities and authentication.

## 2.6 Table Management

This specification defines the mandatory tables and mandatory/optional table rows delivered by the Storage Device manufacturer. The creation or deletion of tables after manufacturing is outside the scope of this specification. The creation or deletion of table rows post-manufacturing is outside the scope of this specification.

## 2.7 Access Control & Personalization

Initial access control policies are preconfigured at Storage Device manufacturing time on manufacturer created SPs. An Opal SSC compliant Storage Device SHALL support personalization of certain Access Control Elements of the Locking SP.

## 2.8 Issuance

The Locking SP MAY be present in the Storage Device when the Storage Device leaves the manufacturer. The issuance of SPs is outside the scope of this specification.

## 2.9 SSC Discovery

Refer to [2] for details (see section 3.1.1).

## 2.10 Mandatory Feature Sets

An Opal SSC compliant Storage Device SHALL support the following TCG Storage Feature Sets:

1) Additional DataStore Tables, Opal Family Feature Set (refer to [6]); 2) PSID, Opal SSC Feature Set (refer to [6]).

3) Block SID Authentication Feature Set (refer to [8])

# 3 Opal SSC Features

## 3.1 Security Protocol 1 Support

### 3.1.1 Level 0 Discovery (M)

Refer to [2] for more details.

An Opal SSC compliant Storage Device SHALL return the following Level 0 response:

- Level 0 Discovery Header (see Table 2)
- TPer feature descriptor (see Table 3)
- Locking feature descriptor (see Table 4)
- Opal SSC V2 feature descriptor (see Table 6)

Additionally, an Opal SSC compliant Storage Device MAY return the following Level 0 response:

- Geometry Reporting feature descriptor (see Table 5)
- Supported Data Removal Mechanism feature descriptor (see Table 8)

#### 3.1.1.1 Level 0 Discovery Header

**Table 2 - Level 0 Discovery Header**

| Bit  <br> Byte | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) |  |  | Length of Parameter Dat |  | a |  |  |
| 1 |  |  |  |  |  |  |  |  |
| 2 |  |  |  |  |  |  |  |  |
| 3 |  |  |  |  |  |  |  | (LSB) |
| 4 | (MSB) |  |  | Data structure revision |  |  |  |  |
| 5 |  |  |  |  |  |  |  |  |
| 6 |  |  |  |  |  |  |  |  |
| 7 |  |  |  |  |  |  |  | (LSB) |
| 8 - 15 |  |  |  | Reserved |  |  |  |  |
| 16 | (MSB) |  |  | Vendor Specific |  |  |  |  |
| … |  |  |  |  |  |  |  |  |
| 47 |  |  |  |  |  |  |  | (LSB) |

An Opal SSC compliant Storage Device SHALL return the following:

- Length of parameter data = VU
- Data structure revision = 0x00000001 or

any version that supports the defined features in this SSC

- Vendor Specific = VU

#### 3.1.1.2 TPer Feature (Feature Code = 0x0001)

**Table 3 - Level 0 Discovery - TPer Feature Descriptor**

| Bit  <br> Byte | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) |  |  | Feature Code (0x0001) |  |  |  |  |
| 1 |  |  |  |  |  |  |  | (LSB) |
| 2 |  | Version |  |  |  | Reserved |  |  |
| 3 |  |  |  | Length |  |  |  |  |
| 4 | Reserved | ComID  <br> Mgmt  <br> Supported | Reserved | Streaming Supported | Buffer Mgmt Supported | ACK/NAK  <br> Supported | Async Supported | Sync Supported |
| 5 - 15 |  |  |  | Reserved |  |  |  |  |

An Opal SSC compliant Storage Device SHALL return the following:

| •  <br> •  <br> •  <br> • | Feature Code Version Length  <br> ComID Mgmt Supported | = 0x0001  <br> = 0x1 or any version that supports the defined features in this SSC  <br> = 0x0C  <br> = VU |
| --- | --- | --- |
| • | Streaming Supported | = 1 |
| • | Buffer Mgmt Supported | = VU |
| • | ACK/NACK Supported | = VU |
| • | Async Supported | = VU |
| • | Sync Supported | = 1 |

#### 3.1.1.3 Locking Feature (Feature Code = 0x0002)

** means the present current state of the respective feature

**Table 4 - Level 0 Discovery - Locking Feature Descriptor**

| Bit Byte | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) |  |  | Feature Code (0x0002) |  |  |  |  |
| 1 |  |  |  |  |  |  |  | (LSB) |
| 2 |  | Version |  |  |  | Reserved |  |  |
| 3 |  |  |  | Length |  |  |  |  |
| 4 | HW Reset  <br> for  <br> LOR/DOR  <br> Supported | MBR  <br> Shadowing Not  <br> Supported | MBR Done | MBR  <br> Enabled | Media Encryption | Locked | Locking Enabled | Locking Supported |
| 5 - 15 |  |  |  | Reserved |  |  |  |  |

An Opal SSC compliant Storage Device SHALL return the following:

- Feature Code = 0x0002
- Version = 0x3 or any version that supports the defined features in this SSC
- Length = 0x0C
- HW Reset for LOR/DOR Supported = VU
- MBR Shadowing Not Supported = 0 o MBR Shadowing feature SHALL be supported. See section 4.3.5.4.
- MBR Done = **
- MBR Enabled = **
- Media Encryption = 1
- Locked = **
- Locking Enabled = See section 3.1.1.3.1
- Locking Supported = 1

##### 3.1.1.3.1 LockingEnabled Definition

The definition of the LockingEnabled bit is changed from [2] as follows:

The LockingEnabled bit SHALL be set to one if an SP that incorporates the Locking template is in any state other than Nonexistent or Manufactured-Inactive; otherwise, the LockingEnabled bit SHALL be set to zero.

#### 3.1.1.4 Geometry Reporting Feature (Feature Code = 0x0003)

##### 3.1.1.4.1 Overview

This information indicates support for logical block and physical block geometry. This feature MAY be returned in the Level 0 Discovery response. See [2] for additional information.

**Table 5 - Level 0 Discovery - Geometry Reporting Feature Descriptor**

| Bit Byte | 7 | 6 |  | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) |  |  |  | Feature Code (0x0003) |  |  |  |  |
| 1 |  |  |  |  |  |  |  |  | (LSB) |
| 2 |  | Version |  |  |  |  | Reserved |  |  |
| 3 |  |  |  |  | Length |  |  |  |  |
| 4 |  |  |  |  | Reserved |  |  |  | ALIGN |
| 5 - 11 |  |  |  |  | Reserved |  |  |  |  |
| 12 | (MSB) |  |  |  | LogicalBlockSize |  |  |  |  |
| 13 |  |  |  |  |  |  |  |  |  |
| 14 |  |  |  |  |  |  |  |  |  |
| 15 |  |  |  |  |  |  |  |  | (LSB) |
| 16 | (MSB) |  |  |  | AlignmentGranularity |  |  |  |  |
| 17 |  |  |  |  |  |  |  |  |  |
| 18 |  |  |  |  |  |  |  |  |  |
| 19 |  |  |  |  |  |  |  |  |  |
| 20 |  |  |  |  |  |  |  |  |  |
| Bit Byte | 7 | 6 | 5 |  | 4 | 3 | 2 | 1 | 0 |
| 21 |  |  |  |  |  |  |  |  |  |
| 22 |  |  |  |  |  |  |  |  |  |
| 23 |  |  |  |  |  |  |  |  | (LSB) |
| 24 | (MSB) |  |  |  | LowestAlignedLBA |  |  |  |  |
| 25 |  |  |  |  |  |  |  |  |  |
| 26 |  |  |  |  |  |  |  |  |  |
| 27 |  |  |  |  |  |  |  |  |  |
| 28 |  |  |  |  |  |  |  |  |  |
| 29 |  |  |  |  |  |  |  |  |  |
| 30 |  |  |  |  |  |  |  |  |  |
| 31 |  |  |  |  |  |  |  |  | (LSB) |

An Opal SSC compliant Storage Device SHALL return the following:

- Feature Code = 0x0003
- Version = 0x01
- Length = 0x1C

##### 3.1.1.4.2 ALIGN

If the value of the AlignmentRequired column of the LockingInfo table is TRUE, then the ALIGN bit shall be set to one. If the value of the AlignmentRequired column of the LockingInfo table is FALSE, then the ALIGN bit shall be cleared to zero.

##### 3.1.1.4.3 LogicalBlockSize

LogicalBlockSize SHALL be set to the value of the LogicalBlockSize column in the LockingInfo table.

##### 3.1.1.4.4 AlignmentGranularity

AlignmentGranularity SHALL be set to the value of the AlignmentGranularity column in the LockingInfo table.

##### 3.1.1.4.5 LowestAlignedLBA

LowestAlignedLBA SHALL be set to the value of the LowestAlignedLBA column in the LockingInfo table.

#### 3.1.1.5 Opal SSC V2 Feature (Feature Code = 0x0203)

**Table 6 - Level 0 Discovery - Opal SSC V2 Feature Descriptor**

| Bit Byte | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) Feature Code (0x0203) |  |  |  |  |  |  |  |
| 1 | (LSB) |  |  |  |  |  |  |  |
| 2 | Feature Descriptor Version Number SSC Minor Version Number |  |  |  |  |  |  |  |
| 3 | Length |  |  |  |  |  |  |  |
| 4 | (MSB) Base ComID |  |  |  |  |  |  |  |
| 5 | (LSB) |  |  |  |  |  |  |  |
| 6 | (MSB) Number of ComIDs |  |  |  |  |  |  |  |
| 7 | (LSB) |  |  |  |  |  |  |  |
| 8 | Reserved for future common SSC parameters |  |  |  |  |  |  | Range  <br> Crossing Behavior |
| 9 | (MSB) Number of Locking SP Admin Authorities Supported |  |  |  |  |  |  |  |
| 10 | (LSB) |  |  |  |  |  |  |  |
| 11 | (MSB) Number of Locking SP User Authorities Supported |  |  |  |  |  |  |  |
| 12 | (LSB) |  |  |  |  |  |  |  |
| 13 | Initial C_PIN_SID PIN Indicator |  |  |  |  |  |  |  |
| 14 | Behavior of C_PIN_SID PIN upon TPer Revert |  |  |  |  |  |  |  |
| 15 - 19 | Reserved for future common SSC parameters |  |  |  |  |  |  |  |

An Opal SSC compliant Storage Device SHALL return the following:

- Feature Code = 0x0203
- Feature Descriptor Version Number = 0x2 or any version that supports the defined features in this SSC
- SSC Minor Version Number = As specified in Table 7
- Length = 0x10
- Base ComID = VU
- Number of ComIDs = 0x0001 or larger
- Range Crossing Behavior = VU
  - 0 = The Storage Device supports commands addressing consecutive LBAs in more than one LBA range if all the LBA ranges addressed are unlocked. See section 4.3.7.
  - 1 = The Storage Device terminates commands addressing consecutive LBAs in more than one LBA range. See 4.3.7
- Number of Locking SP Admin Authorities = 4 or larger
- Number of Locking SP User Authorities = 8 or larger
- Initial C_PIN_SID PIN Indicator = VU o 0x00 = The initial C_PIN_SID PIN value is equal to the C_PIN_MSID PIN value
  - 0xFF = The initial C_PIN_SID PIN value is VU, and MAY not be equal to the C_PIN_MSID PIN value o 0x01 – 0xFE = Reserved
- Behavior of C_PIN_SID PIN upon TPer Revert = VU
  - 0x00 = The C_PIN_SID PIN value becomes the value of the C_PIN_MSID PIN column after successful invocation of Revert on the Admin SP’s object in the SP table
  - 0xFF = The C_PIN_SID PIN value changes to a VU value after successful invocation of Revert on the

Admin SP’s object in the SP table, and MAY not be equal to the C_PIN_MSID PIN value

- 0x01 – 0xFE = Reserved

**Table 7 - SSC Minor Versions**

| SSC Minor  <br> Version  <br> Number | Specification Referenced |
| --- | --- |
| 0x0 | TCG Opal SSC Specification v2.00 |
| 0x1 | TCG Opal SSC Specification v2.01 |
| 0x2 | TCG Opal SSC Specification v2.02 |
| 0x3 | TCG Opal SSC Specification v2.30 |
| All others | Reserved |

If an Opal v2.00 SSC implementation is backward compatible with Opal v1.00, then the Storage Device SHALL also report the Opal SSC feature descriptor as defined in [5].

**Start of Informative Comment**

An Opal v2.00 implementation is backward compatible to Opal v1.00 only if the geometry reported by the Geometry Reporting feature does not specify any alignment restrictions (i.e. ALIGN = FALSE, see section 3.1.1.4.2) , and if the TPer does not specify any granularity restrictions for byte tables (i.e. MandatoryWriteGranularity = 1 for all byte tables, see section 5.3.1.1), and if the “Initial C_PIN_SID PIN Indicator” and “Behavior of C_PIN_SID PIN upon TPer Revert” fields are both 0x00.

**End of Informative Comment**

##### 3.1.1.5.1 Base ComID

The Base ComID field provides the lowest static, pre-assigned ComID.

##### 3.1.1.5.2 Number of ComIDs

The Number of ComIDs field provides the number of static, pre-assigned ComIDs.

#### 3.1.1.6 Supported Data Removal Mechanism Feature (Feature Code = 0x0404)

This feature MAY be returned in the Level 0 Discovery response.

**Table 8 - Level 0 Discovery – Supported Data Removal Mechanism Feature Descriptor**

| Bit Byte | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | (MSB) Feature Code (0x0404) |  |  |  |  |  |  |  |
| 1 | (LSB) |  |  |  |  |  |  |  |
| 2 | Version |  |  |  | Reserved |  |  |  |
| 3 | Length |  |  |  |  |  |  |  |
| 4 | Reserved |  |  |  |  |  |  |  |
| 5 | Reserved |  |  |  |  |  | Data  <br> Removal  <br> Operation  <br> Interrupted | Data  <br> Removal  <br> Operation  <br> Processing |
| 6 | Supported Data Removal Mechanism |  |  |  |  |  |  |  |
| 7 | Reserved |  | Data  <br> Removal  <br> Time  <br> Format for  <br> Bit 5 | Reserved |  | Data  <br> Removal  <br> Time  <br> Format for  <br> Bit 2 | Data  <br> Removal  <br> Time  <br> Format for  <br> Bit 1 | Data  <br> Removal  <br> Time  <br> Format for  <br> Bit 0 |
| 8 - 9 | Data Removal Time for Supported Data Removal Mechanism Bit 0 |  |  |  |  |  |  |  |
| 10 - 11 | Data Removal Time for Supported Data Removal Mechanism Bit 1 |  |  |  |  |  |  |  |
| 12 - 13 | Data Removal Time for Supported Data Removal Mechanism Bit 2 |  |  |  |  |  |  |  |
| 14 - 17 | Reserved |  |  |  |  |  |  |  |
| 18 - 19 | Data Removal Time for Supported Data Removal Mechanism Bit 5 |  |  |  |  |  |  |  |
| 20 - 35 | Reserved for future Supported Data Removal Mechanism parameters |  |  |  |  |  |  |  |

An Opal Compliant Storage Device SHALL return the parameters listed in Table 9:

**Table 9 - Parameter explanation**

| Parameter | Value | Details |
| --- | --- | --- |
| Feature code | 0x0404 | Feature code value |
| Version | 0x02 | Version of the descriptor |
| Length | 0x20 | Length of the feature descriptor |
| Data Removal  <br> Operation  <br> Processing |  | see section 3.1.1.6.1 |
| Data Removal  <br> Operation Interrupted |  | see section 3.1.1.6.2 |
| Reserved |  | Return all zeros |
| Supported Data  <br> Removal  <br> Mechanism |  | see section 3.1.1.6.3 |
| Data Removal Time Format for each bit |  | see section 3.1.1.6.4 |

##### 3.1.1.6.1 Data Removal Operation Processing Definition

The Data Removal Operation Processing bit SHALL be set to one if the TPer is performing any supported data removal operation including:

- Revert,
- RevertSP, or
- GenKey.

Otherwise, the Data Removal Operation Processing bit SHALL be set to zero. If the operation is in progress, the security transport commands such as the security send, and the security receive SHALL be processed by the Storage Device. The Data Removal Operation Processing bit SHALL be set to zero upon a successful completion of a data removal operation.

The Data Removal Operation Processing bit SHALL be set to one if the data removal operation is restarted after a Power Cycle (see Table 16).

##### 3.1.1.6.2 Data Removal Operation Interrupted

The Data Removal Operation Interrupted bit SHALL be set to one if a previously issued data removal operation such as Revert, RevertSP or GenKey was interrupted for any reason (including, power loss, interface reset, etc.). The Data Removal Operation Interrupted bit SHALL be set to zero after successful completion of a data removal operation.

**Start of Informative Comment**

The host can reissue a data removal operation that was interrupted (such as RevertSP, Revert, or GenKey), The Storage Device can be in a locked state if the operation was interrupted and the Storage Device is now operational. **End of Informative Comment**

##### 3.1.1.6.3 Supported Data Removal Mechanism Definition

Each bit of the Supported Data Removal Mechanism (see Table 10) SHALL be set to one if the TPer supports the corresponding Data Removal Mechanism; otherwise, each bit SHALL be set to zero. The TPer SHALL support the

Crypto Erase mechanism and MAY support the Overwrite Data Erase or Block Erase or other mechanisms. The TPer MAY support multiple Data Removal Mechanisms described in Table 10. After a RevertSP method has completed without an error, the condition of user data SHALL be indicated as specified in Table 10.

**Table 10 - Supported Data Removal Mechanism**

| Bit | Name | Condition of user data after Data Removal |
| --- | --- | --- |
| 0 | Overwrite Data Erase1 | The Overwrite Data Erase mechanism causes TPer to alter information by writing a vendor specific data pattern to the medium. |
| 1 | Block Erase1 | The Block Erase mechanism causes the TPer to alter information by setting the physical blocks to a vendor specific value. |
| 2 | Cryptographic Erase 2 | The TPer SHALL support this data erasure mechanism.  Further this mechanism SHALL be executed in addition to any other supported data removal mechanism that is being executed. This bit MAY be used by the Revert or the RevertSP or the GenKey (band erase) mechanisms of data removal, where the cryptographic keys used to encrypt the user data are changed. |
| 3-4 | Reserved | Reserved |
| 5 | Vendor Specific Erase1 | The Vendor Specific Erase mechanisms cause all user data to be removed by a vendor specific method. 3 |
| 6-7 | Reserved |  |
| Notes:  <br> The cryptographic erase operation SHALL also be performed when any of the other data removal mechanisms are used.  <br>   <br> The Cryptographic Erase bit may be used by the Revert, the RevertSP or the GenKey operations (band erase). Any subsequent operation(s) such as Deallocate, or Unmap, or Trim, that is part of the implementation of the data removal operation SHALL be accounted for in the time reported for this operation (see section 3.1.1.6.4). The time value reported SHALL correspond to the estimated completion time of the Cryptographic Erase. For the erase (GenKey) operation, the reported estimated time value will correspond to the estimated completion time of the erase operation, regardless of the extent of the range being erased.  <br>   <br> If a Storage Device supports more than one vendor proprietary method of data removal, then the associated estimated time value will represent the completion time for the longest vendor specific erase mechanism of data removal, then the associated estimated time value will represent the completion time for the longest of the vendor specific mechanisms. |  |  |

##### 3.1.1.6.4 Data Removal Time Format and Data Removal Time Definition

Each Data Removal Time field provides the worst case estimate of the time required to perform the erasure corresponding to each Data Removal Mechanism defined in the Supported Data Removal Mechanism field. The Data Removal Time Format bit identifies the format used to express the time as follows:

1. if the Data Removal Time Format bit is set to zero, then the estimated time is defined in Table 11; and
2. if the Data Removal Time Format bit is set to one, then the estimated time is defined in Table 12.

The Data Removal Time Format bit and Data Removal Time Format field are defined in Table 11 and Table 12.

**Table 11 - Data Removal Time (Data Removal Time Format bit= 0)**

| Value | Time |
| --- | --- |
| 0 | Not reported |
| 1..65534 | (Value x 2) seconds |
| 65535 | >= 131068 seconds |

**Table 12 - Data Removal Time (Data Removal Time Format bit= 1)**

| Value | Time |
| --- | --- |
| 0 | Not reported |
| 1..65534 | (Value x 2) minutes |
| 65535 | >= 131068 minutes |

**Start of Informative Comment**

Each Data Removal Time field gives an estimate of the total time required to perform the erasure for each corresponding Data Removal Mechanism. This field is not a dynamic estimate of the remaining time for completion. When GenKey is performed on a range that’s less than the global range, the time needed for the completion of the operation can be less than the time reported for the operation. The reported estimated time for the data removal operation will be for the entire capacity of the Storage Device. The host software can use the ratio of the band size to the entire capacity of the Storage Device, to derive the estimated time for erasing a band. **End of Informative Comment**

## 3.2 Security Protocol 2 Support

### 3.2.1 ComID Management

ComID management support is reported in Level 0 Discovery. Statically allocated ComIDs are also discoverable via the Level 0 Discovery response.

### 3.2.2 Stack Protocol Reset (M)

An Opal SSC compliant Storage Device SHALL support the Stack Protocol Reset command. Refer to [2] for details.

### 3.2.3 TPER_RESET command (M)

If the TPER_RESET command is enabled, it SHALL cause the following before the TPer accepts the next IF-SEND or IF-RECV command:

1. all dynamically allocated ComIDs SHALL return to the Inactive state;
2. all open sessions SHALL be aborted on all ComIDs;
3. all uncommitted transactions SHALL be aborted on all ComIDs;
4. the synchronous protocol stack for all ComIDs SHALL be reset to its initial state
5. all TCG command and response buffers SHALL be invalidated for all ComIDs;
6. all related method processing occurring on all ComIDs SHALL be aborted;
7. The TPer’s knowledge of the host’s communications capabilities, on all ComIDs, SHALL be reset to the initial minimum assumptions defined in [2] or the TPer’s SSC definition;
8. the values of the ReadLocked and WriteLocked columns SHALL be set to True for all Locking SP’s Locking objects that contain the Programmatic enumeration value in the LockedOnReset column;
9. the value of the Done column of the Locking SP’s MBRControl table SHALL be set to False, if the DoneOnReset column contains the Programmatic enumeration value.

The TPER_RESET command is delivered by the transport IF-SEND command. If the TPER_RESET command is enabled, the TPer SHALL accept and acknowledge it at the interface level. If the TPER_RESET command is disabled, the TPer SHALL abort it at the interface level with the “Other Invalid Command Parameter” status (see [4]). There is no IF-RECV response to the TPER_RESET command.

The TPER_RESET command is defined in Table 13.

The Transfer Length SHALL be non-zero. All data transferred SHALL be ignored.

**Table 13 - TPER_RESET Command**

| FIELD | VALUE |
| --- | --- |
| Command | IF-SEND |
| Protocol ID | 0x02 |
| Transfer Length | Non-zero |
| ComID | 0x0004 |

## 3.3 Communications

### 3.3.1 Communication Properties

The TPer SHALL support the minimum communication buffer size as defined in section 4.1.1.1. For each ComID, the physical buffer size SHALL be reported to the host via the Properties method.

The TPer SHALL terminate any IF-SEND command whose transfer length is greater than the reported MaxComPacketSize size for the corresponding ComID. For details, refer to “Invalid Transfer Length parameter on IFSEND” in [4].

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

- terminate the command as defined in [4] with “Other Invalid Command Parameter”, or
- follow the requirements defined in [2] for “IF-SEND to Inactive or Unsupported Reserved ComID” or “IF-RECV to Inactive or Unsupported Reserved ComID”.

ComIDs SHALL be assigned based on the allocation presented in Table 14.

**Table 14 - ComID Assignments**

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

The TPer SHALL support tokens listed in Table 15. If an unsupported token is encountered, the TPer SHALL treat the token as a streaming protocol violation and return an error per the definition in section 3.3.4.1.3.

**Table 15 - Supported Tokens**

| Token | Acronym |
| --- | --- |
| Tiny atom | N/A |
| Short atom | N/A |
| Medium atom | N/A |
| Long atom | N/A |
| Start List | SL |
| End List | EL |
| Start Name | SN |
| End Name | EN |
| Call | CALL |
| End of Data | EOD |
| End of session | EOS |
| Start transaction | ST |
| End of transaction | ET |
| Empty atom | MT |

The TPer SHALL support the above token atoms with the B bit set to zero or one and the S bit set to zero.

##### 3.3.4.1.2 TCG Packets

Within a single IF-SEND/IF-RECV command, the TPer SHALL support a ComPacket containing one Packet, which contains one Subpacket. The host may discover TPer support of capabilities beyond this requirement in the parameters returned in response to a Properties method.

The TPer MAY ignore Credit Control Subpackets sent by the host. The host may discover TPer support of Credit Management with Level 0 Discovery. For more details refer to Section 3.1.1 Level 0 Discovery (M)

The TPer MAY ignore the AckType and Acknowledgement fields in the Packet header on commands from the host and set these fields to zero in its responses to the host. The host may discover TPer support of the TCG packet acknowledgement/retry mechanism with Level 0 Discovery. For more details refer to Section 3.1.1 Level 0 Discovery

(M)

The TPer MAY ignore packet sequence numbering and not enforce any sequencing behavior. Refer to [2] for details on discovery of packet sequence numbering support.

##### 3.3.4.1.3 Payload Error Response

The TPer SHALL respond according to the following rules if it encounters a streaming protocol violation:

- If the error is on Session Manager or is such that the TPer cannot resolve a valid session ID from the payload (i.e. errors in the ComPacket header or Packet header), then the TPer SHALL discard the payload and immediately transition to the “Awaiting IF-SEND” state.
- If the error occurs after the TPer has resolved the session ID, then the TPer SHALL abort the session and MAY prepare a CloseSession method for retrieval by the host.

### 3.3.5 Storage Device Resets

#### 3.3.5.1 Interface Resets

Interface resets that generate TCG reset events are defined in [4].

Interface initiated TCG reset events SHALL result in:

1. All open sessions SHALL be aborted;
2. All uncommitted transactions SHALL be aborted;
3. All pending session startup activities SHALL be aborted;
4. All TCG command and response buffers SHALL be invalidated;
5. All related method processing SHALL be aborted;
6. For each ComID, the state of the synchronous protocol stack SHALL transition to “Awaiting IF-SEND” state;
7. No notification of these events SHALL be sent to the host.

#### 3.3.5.2 TCG Reset Events

Table 16 replaces the definition of TCG reset_types that are defined in [2]: **Table 16 - reset_types**

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

# 4 Opal SSC-compliant Functions and SPs

## 4.1 Session Manager

### 4.1.1 Methods

#### 4.1.1.1 Properties (M)

An Opal SSC compliant Storage Device SHALL support the Properties method. The requirements for support of the various TPer and Host properties, and the requirements for their values, are shown in Table 17. **Table 17 - Properties Requirements**

| Property Name | TPer Property Requirements and Values Reported | Host Property Requirements and Values Accepted |
| --- | --- | --- |
| MaxComPacketSize | (M)  <br> 2048 minimum | (M)  <br> Initial Assumption: 2048 Minimum allowed: 2048  <br> Maximum allowed: VU |
| MaxResponseComPacketSize | (M)  <br> 2048 minimum | (N)  <br> Although this is a legal host property, there is no requirement for the TPer to use it.  The TPer MAY ignore this host property and not list it in the  <br> HostProperties result of the  <br> Properties method response. |
| MaxPacketSize | (M)  <br> 2028 minimum | (M)  <br> Initial Assumption: 2028 Minimum allowed: 2028  <br> Maximum allowed: VU |
| MaxIndTokenSize | (M)  <br> 1992 minimum | (M)  <br> Initial Assumption: 1992 Minimum allowed: 1992  <br> Maximum allowed: VU |
| MaxPackets | (M) 1 minimum | (M)  <br> Initial Assumption: 1 Minimum allowed: 1  <br> Maximum allowed: VU |
| MaxSubpackets | (M) 1 minimum | (M) |
|  |  | Initial Assumption: 1 Minimum allowed: 1  <br> Maximum allowed: VU |
| MaxMethods | (M) 1 minimum | (M)  <br> Initial Assumption: 1 Minimum allowed: 1  <br> Maximum allowed: VU |
| MaxSessions | (M) 1 minimum | N/A – not a host property |
| MaxAuthentications | (M) 2 minimum | N/A – not a host property |
| MaxTransactionLimit | (M) 1 minimum | N/A – not a host property |
| DefSessionTimeout | (M)  <br> VU | N/A – not a host property |

#### 4.1.1.2 StartSession (M)

An Opal SSC compliant Storage Device SHALL support the following parameters for the StartSession method:

- HostSessionID
- SPID
- Write
- HostChallenge
- HostSigningAuthority

For an Opal SSC compliant Storage Device, a value of “True” for the Write parameter SHALL be supported.

For an Opal SSC compliant Storage Device, a value of “False” (i.e. read only session) for the Write parameter may or may not be supported.

The SessionTimeout parameter of the StartSession method is optional. It may be used by the host to specify a timeout value for the session. Refer to [2] for details.

If a TPer supports the optional SessionTimeout parameter and the parameter is included in the invocation of the StartSession method, then its value is permitted if it satisfies the following conditions:

1. It is less than or equal to the TPer’s MaxSessionTimeout property, the property is zero, or the property is not defined.
2. It is less than or equal to the value of the SPSessionTimeout column in the SP’s SPInfo table, the value is zero, or the column is empty; and
3. It is greater than or equal to the TPer’s MinSessionTimeout property, or the property is not defined.

If the specified SesstionTimeout parameter value does not satisfy conditions a), b), and c) above, the TPer SHALL fail the StartSession method as defined in [2].

#### 4.1.1.3 SyncSession (M)

An Opal SSC compliant Storage Device SHALL support the following parameters for the SyncSession method:

- HostSessionID
- SPSessionID

#### 4.1.1.4 CloseSession (O)

An Opal SSC compliant Storage Device MAY support the CloseSession method.

## 4.2 Admin SP

The Admin SP includes the Base Template and the Admin Template.

### 4.2.1 Base Template Tables

All tables included in the following subsections are Mandatory.

#### 4.2.1.1 SPInfo (M)

The SPInfo table is defined in [2], and Table 18 defines the Preconfiguration Data for the SPInfo table.

**Table 18 - Admin SP - SPInfo Table Preconfiguration**

| UID | SPID | Name | Size | SizeInUse | SPSessionTimeout | Enabled |
| --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 02  <br> 00 00 00 01 | 00 00 02 05 00 00 00 01 | “Admin” |  |  |  | T |

As specified in [2], a TPer SHALL ignore the value of SPSessionTimeout column if:

1. no value exists in SPSessionTimeout column; or
2. SPSessionTimeout column is zero.

#### 4.2.1.2 SPTemplates (M)

The SPTemplates table is defined in [2], and Table 19 defines the Preconfiguration Data for the SPTemplates table.

*ST1 means this version number or any version number that complies with this SSC.

**Table 19 - Admin SP - SPTemplates Table Preconfiguration**

| UID | TemplateID | Name | Version |
| --- | --- | --- | --- |
| 00 00 00 03  <br> 00 00 00 01 | 00 00 02 04 00 00 00 01 | “Base” | 00 00 00 02  <br> *ST1 |
| 00 00 00 03  <br> 00 00 00 02 | 00 00 02 04 00 00 00 02 | “Admin” | 00 00 00 02  <br> *ST1 |

#### 4.2.1.3 Table (M)

The Table table is defined in [2], and Table 20 defines the Preconfiguration Data for the Table table.

Refer to section 5.3 for a description and requirements of the MandatoryWriteGranularity and RecommendedAccessGranularity columns.

If the Data Removal Mechanism feature descriptor is not supported, then the DataRemovalMechanism row SHALL NOT exist.

**Table 20 - Admin SP - Table Table Preconfiguration**

| UID | Name | CommonName | TemplateID | Kind | Column | NumColumns | Rows | RowsFree | RowBytes | LastID | MinSize | M axSize | MandatoryWrite Granularity | RecommendedAccess Granularity |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00  <br> 00 01  <br> 00 00 00 01 | “Table” |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00 00 02 | “SPInfo” |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00 00 03 | “SPTemplates” |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00 00 06 | "MethodID" |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00 00 07 | "AccessControl" |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| UID | Name | CommonName | TemplateID | Kind | Column | NumColumns | Rows | RowsFree | RowBytes | LastID | MinSize | M axSize | MandatoryWrite Granularity | RecommendedAccess Granularity |
| 00 00  <br> 00 01  <br> 00 00 00 08 | "ACE" |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00 00 09 | "Authority" |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00  <br> 00 0B | "C_PIN" |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00 02 01 | "TPerInfo" |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00 02 04 | "Template" |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00 02 05 | "SP" |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00  <br> 00 01  <br> 00 00 11 01 | DataRemovalMecha nism” |  |  | Obje ct |  |  |  |  |  |  |  |  | 0 | 0 |

**Start of Informative Comment**

[2] states, “The Table table in the Admin SP includes a row for each table that the TPer supports, in addition to a row for each table that exists in the Admin SP.” However, the Opal SSC requires only the tables from the Admin SP to be included in the Admin SP’s Table table, as indicated in Table 20.

**End of Informative Comment**

#### 4.2.1.4 MethodID (M)

The MethodID table is defined in [2], and Table 21 defines the Preconfiguration Data for the MethodID table.

*MT1: refer to section 5.1.2 for details on the requirements for supporting Revert.

*MT2: refer to section 5.1.1 for details on the requirements for supporting Activate.

.

**Table 21 - Admin SP - MethodID Table Preconfiguration**

| UID | Name | CommonName | TemplateID |
| --- | --- | --- | --- |
| 00 00 00 06  <br> 00 00 00 08 | "Next" |  |  |
| 00 00 00 06  <br> 00 00 00 0D | "GetACL" |  |  |
| 00 00 00 06  <br> 00 00 00 16 | "Get" |  |  |
| 00 00 00 06  <br> 00 00 00 17 | "Set" |  |  |
| 00 00 00 06  <br> 00 00 00 1C | "Authenticate" |  |  |
| 00 00 00 06 00 00 02 02  <br>        *MT1 | "Revert" |  |  |
| 00 00 00 06  <br> 00 00 02 03  <br> *MT2 | "Activate" |  |  |
| 00 00 00 06  <br> 00 00 06 01 | “Random” |  |  |

#### 4.2.1.5 AccessControl (M)

Table 22 contains Optional rows identified by (O).

Notation:

*AC1: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the Table object UIDs

*AC2: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the SPTemplates object UIDs *AC3: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the MethodID object UIDs

*AC4: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the ACE object UIDs

*AC5: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the Authority object UIDs

*AC6: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the Template object UIDs

*AC7: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the SP object UIDs

**Start of Informative Comment**

*AC8: refer to section 5.1.2 for details on the requirements for supporting Revert

*AC9: refer to section 5.1.1 for details on the requirements for supporting Activate

**End of Informative Comment**

The InvokingID, MethodID and GetACLACL columns are a special case. Although they are marked as Read-Only with fixed access control, the access control for invocation of the Get method is (N).

The ACL column is readable only via the GetACL method.

**Table 22 - Admin SP - AccessControl Table Preconfiguration**

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Table |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 01 00 00 00 00 00 00 | Table | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 01 00 00 TT TT TT TT *AC1 | TableObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| SPInfo |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 02 00 00 00 01 00 00 | SPInfoObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| SPTemplates |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 03 00 00 00 00 00 00 | SPTemplates | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 03 00 00 TT TT TT TT *AC2 | SPTemplatesObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| MethodID |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 06 00 00 00 00 00 00 | MethodID | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 06 00 00 TT TT TT TT *AC3 | MethodIDObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| ACE |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 00 00 | ACE | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 TT TT TT TT *AC4 | ACEObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Authority |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 00 00 00 | Authority | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 TT TT TT TT *AC5 | AuthorityObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 09 00 00 00 03 00 00 | Makers | Set |  | ACE_Set_Enabled |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 09 00 00 0 0 0 2 01 | Admin1 | Set |  | ACE_Set_Enabled |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 0 0 0 2 0 0 (+ ) XX | AdminXX | Set |  | ACE_Set_Enabled |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C_PIN |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 00 00 00 | C_PIN | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 00 00 | C_PIN_SID | Get |  | ACE_C_PIN_SID_Get_NOPIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 00 00 | C_PIN_SID | Set |  | ACE_C_PIN_SID_Set_PI N |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 02 00 84 | C_PIN_MSID | Get |  | ACE_C_PIN_MSID_Get_PIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 00 00 0B 00 01 00 02 | C_PIN_Admin1 | Get |  | ACE_C_PIN_SID_Get_NOPIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 00 00 02 (+ ) XX | C_PIN_AdminXX | Get |  | ACE_C_PIN_SID_Get_NOPIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 00 02 01 | C_PIN_Admin1 | Set |  | ACE_C_PIN_Admins_Set_PI N |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 00 00 02 (+ ) XX | C_PIN_AdminXX | Set |  | ACE_C_PIN_Admins_Set_PI N |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| TPerInfo |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 01 00 02 00 01 03 00 | TPerInfoObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 01 00 02 00 01 03 00 | TPerInfoObj | Set |  | ACE_TPerInfo_Set_ProgrammaticResetEnabl e |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Template |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 04 00 02 00 00 00 00 | Template | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 02 04 TT TT TT TT *AC6 | TemplateObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| SP |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 00 00 01 00 00 | ThisSP | Authenticate |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 00 00 01 00 00 | ThisSP | Random |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 02 05 00 00 00 00 | SP | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 00 02 05 TT TT TT TT *AC7 | SPObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC8 |  | 00 00 02 05 TT TT TT TT *AC7 | SPObj | Revert |  | ACE_SP_SID, ACE_Admin |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC9 |  | 00 00 02 05 TT TT TT TT *AC7 | SPObj | Activate |  | ACE_SP_SID |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| DataRemovalM echanism |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 01 00 11 00 01 00 00 | DataRemovalMec hanismObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Table association - Informative text | UID | InvokingID | InvokingID Name - informative text | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
|  |  | 00 01 00 11 00 01 00 00 | DataRemovalMechanismObj | Set |  | ACE_DataRemovalMechanism_Set_ActiveDataRemovalMechanism |  |  |  | ACE_Anybody |  |  |  |  |  |  |

#### 4.2.1.6 ACE (M)

Table 23 contains Optional rows designated with (O).

**Start of Informative Comment**

*ACE1 means that row is (M) if the TPer supports either Activate or Revert, and (N) otherwise.

**End of Informative Comment**

**Table 23 - Admin SP - ACE Table Preconfiguration**

| Table Association - Informative text | UID | Name | CommonName | BooleanExpr | Columns |
| --- | --- | --- | --- | --- | --- |
| BaseACEs |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 00 00 01 | "ACE_Anybody" |  | Anybody | All |
|  | 00 00 00 08  <br> 00 00 00 02 | "ACE_Admin" |  | Admins | All |
| Table Association - Informative text | UID | Name | CommonName | BooleanExpr | Columns |
| Authority |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 00 01 | "ACE_Set_Enabled" |  | SID | Enabled |
| C_PIN |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 00 8C 02 | "ACE_C_PIN_SID_Get_NOPIN" |  | Admins OR SID | UID, CharSet, TryLimit, Tries, Persistence |
|  | 00 00 00 08  <br> 00 00 8C 03 | "ACE_C_PIN_SID_Set_PIN" |  | SID | PIN |
|  | 00 00 00 08  <br> 00 00 8C 04 | "ACE_C_PIN_MSID_Get_PIN" |  | Anybody | UID, PIN |
|  | 00 00 00 08  <br> 00 03 A0 01 | "ACE_C_PIN_Admins_Set_PIN" |  | Admins OR SID | PIN |
| TPerInfo |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 00 03 | "ACE_TPerInfo_Set_Programmati cResetEnable" |  | SID | ProgrammaticResetEnable |
| SP |  |  |  |  |  |
| *ACE1 | 00 00 00 08  <br> 00 03 00 02 | "ACE_SP_SID" |  | SID | All |
| DataRemovalM echanism |  |  |  |  |  |
| *ACE1 | 00 00 00 08  <br> 00 05 00 01 | "ACE_DataRemovalMechanism_S et_ActiveDataRemovalMechanism <br> " |  | Admins OR SID | ActiveDataRemoval Mechanism |

#### 4.2.1.7 Authority (M)

The Authority table is defined in [2], and Table 24 defines the Preconfiguration Data for the Authority table.

Note:

• Admin1 (M) is required; any additional Admin authorities are (O)

**Table 24 - Admin SP - Authority Table Preconfiguration**

| UID | Name | CommonName | IsClass | Class | Enabled | Secure | HashAndSign | PresentCertificate | Operation | Credential | ResponseSign | ResponseExch | ClockStart | ClockEnd | Limit | Uses | Log | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 09 00 00 00 01 | "Anybody" |  | F | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09 00 00 00 02 | "Admins" |  | T | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09 00 00 00 03 | "Makers" |  | T | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09 00 00 00 06 | "SID" |  | F | Null | T | None | None | F | Password | C_PIN_SID | Null | Null |  |  |  |  |  |  |
| 00 00 00 09 00 00 02 01 | "Admin1" |  | F | Admins | F | None | None | F | Password | C_PIN_Admin 1 | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 00 02 00  <br> (+XX)1  <br> (O) | "Admin XX " |  | F | Admins | F | None | None | F | Password | C_PIN_Admin XX | Null | Null |  |  |  |  |  |  |

#### 4.2.1.8 C_PIN (M)

The C_PIN table is defined in [2], and Table 25 defines the Preconfiguration Data for the C_PIN table. **Table 25 - Admin SP - C_PIN Table Preconfiguration**

| UID | Name | CommonName | PIN | CharSet | TryLimit | Tries | Persistence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 0B 00 00 00 01 | "C_PIN_SID" |  | VU | Null | VU | VU | FALSE |
| 00 00 00 0B 00 00 84 02 | "C_PIN_MSID" |  | MSID |  |  |  |  |
| UID | Name | CommonName | PIN | CharSet | TryLimit | Tries | Persistence |
| 00 00 00 0B 00 00 02 01 | "C_PIN_Admin1" |  | “” | Null | 0 | 0 | FALSE |
| 00 00 00 0B  <br> 00 00 02 00  <br> (+XX)  <br> (O) | "C_PIN_AdminXX" |  | “” | Null | 0 | 0 | FALSE |

For Storage Devices that will be used in environments where an automated take ownership process is required, the initial PIN column value of C_PIN_SID SHALL be set to the PIN column value of C_PIN_MSID. In order to allow for alternative take ownership processes, the initial PIN column value of C_PIN_SID MAY be Vendor Unique (VU).

**Start of Informative Comment**

Several activation / take ownership models are possible. The simplest model, which is the only model supported by Opal v1.00, is a process whereby the host discovers the initial C_PIN_SID PIN value by performing a Get operation on the C_PIN_MSID object. This model requires that the initial C_PIN_SID PIN be the value of the C_PIN_MSID PIN. Opal v2.00 allows the initial C_PIN_SID PIN value to be vendor unique in order to allow for alternative activation / take ownership models. Such models require that the C_PIN_SID PIN be retrieved in a way that is beyond the scope of this specification.

Before a device vendor chooses to implement an activation / take ownership model based on a vendor unique SID PIN, the Storage Device vendor must undertake due diligence to ensure that the ecosystem exists to support such an activation / take ownership model. Having a C_PIN_SID PIN value that is different from the C_PIN_MSID PIN value may have serious ramifications, such as the inability to take ownership of the Storage Device.

See section 5.1.2.2.1 for an explanation of how the Revert method affects the value of the C_PIN_SID PIN column. **End of Informative Comment**

### 4.2.2 Base Template Methods

Refer to section 4.2.1.4 for supported methods.

### 4.2.3 Admin Template Tables

#### 4.2.3.1 TPerInfo (M)

The TPerInfo table has the column defined in Table 26, in addition to those defined in [2], and Table 27 defines the Preconfiguration Data for the TPerInfo table:

**Table 26 - Admin SP – TPerInfo Columns**

| Column Number | Column Name | IsUnique | Colum Type |
| --- | --- | --- | --- |
| 0x08 | ProgrammaticResetEnable |  | boolean |

• **ProgrammaticResetEnable**

This column indicates whether support for programmatic resets is enabled or not. If ProgrammaticResetEnable is TRUE, then the TPER_RESET command is enabled. If ProgrammaticResetEnable is FALSE, then the TPER_RESET command is not enabled.

This column is readable by Anybody and modifiable by the SID authority.

*TP1 means that the value in the GUDID column SHALL comply with the format defined in [2].

*TP2 means that this version or any version that supports the defined features in this SSC.

*TP3 means that the SSC column is a list of names and SHALL have “Opal” as one of the list elements.

**Table 27 - Admin SP - TPerInfo Table Preconfiguration**

| UID | Bytes | GUDID | Generation | Firmware Version | ProtocolVersion | SpaceForIssuance | SSC | ProgrammaticResetEnable |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 02 01  <br> 00 03 00 01 |  | VU *TP1 |  |  | 1 *TP2 |  | [“Opal”]  <br> *TP3 | FALSE |

#### 4.2.3.2 Template (M)

The Template table is defined in [2], and Table 28 defines the Preconfiguration Data for the Template table.

**Table 28 - Admin SP - Template Table Preconfiguration**

| UID | Name | Revision Number | Instances | MaxInstances |
| --- | --- | --- | --- | --- |
| 00 00 02 04 00 00 00 01 | "Base" | 1 | VU | VU |
| 00 00 02 04 00 00 00 02 | "Admin" | 1 | 1 | 1 |
| 00 00 02 04 00 00 00 06 | "Locking" | 1 | 1 | 1 |

#### 4.2.3.3 SP (M)

The SP table is defined in [2], and Table 29 defines the Preconfiguration Data for the SP table.

*SP1 means that this row only exists in the Admin SP's OFS when the Locking SP is created by the manufacturer.

**Table 29 - Admin SP - SP Table Preconfiguration**

| UID | Name | ORG | EffectiveAuth | DateOfIssue | Bytes | LifeCycle | Frozen |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 02 05 00 00 00 01 | "Admin" |  |  |  |  | Manufactured | FALSE |
| 00 00 02 05 00 00 00 02  <br>        *SP1 | "Locking" |  |  |  |  | ManufacturedInactive | FALSE |

### 4.2.4 Admin Template Methods

Refer to section 4.2.1.4 for supported methods.

### 4.2.5 Opal Additional Column Types

#### 4.2.5.1 Data_removal_mechanism

The data_removal_mechanism type is defined in Table 30 for Opal:

**Table 30 - data_removal_mechanism Type Table Addition**

| UID | Name | Format |
| --- | --- | --- |
| 00 00 00 05 00 00 04 20 | data_removal_mechanism | Enumeration_Type,  <br> 0,  <br> 7 |

Table 31 defines the enumeration values. The mechanisms associated with each Enumeration Value are defined in Table 10.

**Table 31 - data_removal_mechanism Enumeration Values**

| Enumeration Value | Associated Value |
| --- | --- |
| 0 | Overwrite Data Erase |
| 1 | Block Erase |
| 2 | Cryptographic Erase |
| 3 – 4 | Reserved |
| 5 | Vendor Specific Erase |
| 6-7 | Reserved |

### 4.2.6 Opal Additional Data Structures

#### 4.2.6.1 DataRemovalMechanism (ObjectTable)

The DataRemovalMchanism table is defined in Table 32

| Column Number | Column Name | IsUnique | Column Type |
| --- | --- | --- | --- |
| 0x00 | UID |  | uid |
|  | ActiveDataRemovalMechanism |  | data_removal_mechanism |

**Table 32 - DataRemovalMechansim Table Description** 0x01

##### 4.2.6.1.1 UID

This is the unique identifier of this row in the DataRemovalMechanism table. This column SHALL NOT be modifiable by the host.

##### 4.2.6.1.2 ActiveDataRemovalMechanism

This column value selects which Data Removal Mechanism in the Supported Data Removal Mechanism field in the

Supported Data Removal Mechanism feature descriptor is active and will be used to remove data upon execution of the Revert method, the RevertSP method or the GenKey method. If an attempt is made to set the ActiveDataRemovalMechanism column value to an unsupported value of the data_removal_mechanism type, then the Set method invocation SHALL result in the method failing with the status INVALID_PARAMETER.

### 4.2.7 Opal Additional Tables

#### 4.2.7.1 DataRemovalMechansim (M)

The DataRemovalMechanism table SHALL contain exactly one row with UID = 0x00 00 11 01 00 00 00 01. The DataRemovalMechanism table SHALL be supported (see Table 33).

**Table 33 - Admin SP – DataRemovalMechansim Table Preconfiguration**

| UID | ActiveDataRemovalMechanism |
| --- | --- |
| 00 00 11 01   <br> 00 00 00 01 | VU |

### 4.2.8 Crypto Template Tables

An Opal SSC compliant Storage Device is not required to support any Crypto template tables.

### 4.2.9 Crypto Template Methods

Refer to section 4.2.1.4 for supported methods.

#### 4.2.9.1 Random

The TPer SHALL implement the Random method with the constraints stated in this subsection. TPer support of the following parameters is Mandatory:

• Count

Attempts to use unsupported parameters SHALL result in a method failure response with TCG status INVALID_PARAMETER. The TPer SHALL support Count parameter values less than or equal to 32.

## 4.3 Locking SP

### 4.3.1 Base Template Tables

All tables defined with (M) in section titles are Mandatory.

#### 4.3.1.1 SPInfo (M)

The SPInfo table is defined in [2], and Table 34 defines the Preconfiguration Data for the SPInfo table.

**Table 34 - Locking SP - SPInfo Table Preconfiguration**

| UID | SPID | Name | Size | SizeInUse | SPSessionTimeout | Enabled |
| --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 02  <br> 00 00 00 01 | 00 00 02 05  <br> 00 00 00 02 | "Locking" |  |  |  | T |

#### 4.3.1.2 SPTemplates (M)

The SPTemplates table is defined in [2], and Table 35 defines the Preconfiguration Data for the SPTemplates table. *SP1 means that this version number or any number that supports the defined features in this SSC **Table 35 - Locking SP - SPTemplates Table Preconfiguration**

| UID | TemplateID | Name | Version |
| --- | --- | --- | --- |
| 00 00 00 03 00 00 00 01 | 00 00 02 04 00 00 00 01 | "Base" | 00 00 00 02  <br> *SP1 |
| 00 00 00 03 00 00 00 02 | 00 00 02 04 00 00 00 06 | "Locking" | 00 00 00 02  <br> *SP1 |

#### 4.3.1.3 Table (M)

The Table table is defined in [2], and Table 36 defines the Preconfiguration Data for the Table table. Table 36 contains Optional rows designated with (O).

*TT1 means that only one of the two K_AES* tables is required

Refer to section 5.3 for a description and requirements of the MandatoryWriteGranularity and RecommendedAccessGranularity columns.

| UID | Name | CommonName | TemplateID | Kind | Column | NumColumns | Rows | RowsFree | RowBytes | LastID | MinSize | MaxSize | MandatoryWrite | RecommendedAccess |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 01  <br> 00 00 00 01 | "Table" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 02 | "SPInfo" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 03 | "SPTemplates" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 06 | "MethodID" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 07 | "AccessControl" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 08 | "ACE" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 09 | "Authority" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |

**Table 36 - Locking SP - Table Table Preconfiguration**

| UID | Name | CommonName | TemplateID | Kind | Column | NumColumns | Rows | RowsFree | RowBytes | LastID | MinSize | MaxSize | MandatoryWrite | RecommendedAccess |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 01  <br> 00 00 00 0B | "C_PIN" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 00 1D | "SecretProtect" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 01 | "LockingInfo" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 02 | "Locking" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 03 | "MBRControl" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 04 | "MBR" |  |  | Byte |  |  | 00000 |  |  |  |  |  | VU | VU |
| 00 00 00 01  <br> 00 00 08 05  <br> *TT1 | "K_AES_128” |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 08 06  <br> *TT1 | "K_AES_256" |  |  | Object |  |  |  |  |  |  |  |  | 0 | 0 |
| 00 00 00 01  <br> 00 00 10 01 | "DataStore" |  |  | Byte |  |  | 0x00A00000 min |  |  |  |  |  | VU | VU |

#### 4.3.1.4 Type (N)

The Type table is not required (N) by Opal. The following types as defined by [2] SHALL meet the following requirements:

- The "boolean_ACE" type (00000005 0000040E) SHALL include the OR Boolean operator.
- The "AC_element" type (00000005 00000801) SHALL support at least 23 entries (8 User authorities, 4 Admin authorities, and 11 Boolean operators).

#### 4.3.1.5 MethodID (M)

The MethodID table is defined in [2], and Table 37 defines the Preconfiguration Data for the MethodID table.

*MT1 means refer to section 5.1.2.3 for details on the requirements for supporting RevertSP.

**Table 37 - Locking SP - MethodID Table Preconfiguration**

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

Table 38 contains Optional rows designated with (O).

**Start of Informative Comment**

*AC1: refer to section 5.1.2.3 for details on the requirements for supporting RevertSP

*AC8: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the SecretProtect object UIDs **End of Informative Comment**

*AC2: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the Table object UIDs

*AC3: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the SPTemplates object UIDs

*AC4: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the MethodID object UIDs

*AC5: the notation of “TT TT TT TT” represents a shorthand for the LSBs of the ACE object UIDs

*AC6: only K_AES_128 or K_AES_256 related rows are Mandatory

*AC7: the notation of “TT TT TT TT” represents a shorthand for the LSB of the Authority object UIDs Notes:

- The AccessControl table is different from any other table defined in this specification. Although cells in this table are marked as Read-Only with fixed access control, the access control for invocation of the Get method is (N).
- The ACL column is readable only via the GetACL method.

**Table 38 - Locking SP - AccessControl Table Preconfiguration**

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
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
|  |  | 00 03 00 00 00 00 00 00 | SPTemplates | Next |  | ACE_Anybod y |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID |  | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| <br> |  | 00 03 00 00 TT TT TT TT | *AC3 | SPTemplatesObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| MethodID |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 06 00 00 00 00 00 |  | MethodID | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| <br> |  | 00 06 00 00 TT TT TT TT *AC4 |  | MethodIDObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| ACE |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 00 00 |  | ACE | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| <br> |  | 00 08 00 00 TT TT TT TT *AC5 |  | ACEObj | Get |  | ACE_ACE_Get_All |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 03 80 |  | ACE_ACE_Get_All | Set |  | ACE_ACE_Set_BooleanExpressio n |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 08 00 00 00 00 03 90 | ACE_Authority_Get_All | Set |  | ACE_ACE_Set_BooleanExpressio n |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 03 01 A8 | ACE_C_PIN_User1_Set_PIN | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 ) 03 A8 00 (+MMMM | ACE_C_PIN_User MMMM _Set_PI N | Set |  | ACE_ACE_Set_BooleanExpressio n |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 01 04 40 | ACE_User1_Set_CommonName | Set |  | ACE_ACE_Set_BooleanExpressio n |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 08 00 00 00 0 04 40 0 ) (+MMMM | ACE_User MMMM _Set_CommonName | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC6 |  | 00 08 00 00 00 00 03 B0 | ACE_K_AES_128_GlobalRange_GenKe y | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC6 |  | 00 08 00 00 00 01 03 B0 | ACE_K_AES_128_Range1_GenKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| *AC6 |  | 00 08 00 00 00 ) 03 B0 00 (+NNNN | ACE_K_AES_128_RangeNNNN _GenKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC6 |  | 00 08 00 00 00 00 03 B8 | ACE_K_AES_256_GlobalRange_GenKe y | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| *AC6 |  | 00 08 00 00 00 01 03 B8 | ACE_K_AES_256_Range1_GenKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| *AC6 |  | 00 08 00 00 00 ) 03 B8 00 (+NNNN | ACE_K_AES_256_RangeNNNN _GenKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody | <br>   <br> | <br>   <br> | <br>   <br> | <br>   <br> | <br>   <br> | <br>   <br> |
|  |  | 00 08 00 00 00 00 03 D0 | ACE_Locking_GlobalRange_Get_ RangeStartToActiveKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 03 01 D0 | ACE_Locking_Range1_Get_ RangeStartToActiveKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 08 00 00 00 ) 03 D0 00 (+NNNN | ACE_Locking_RangeNNNN_Get_ RangeStartToActiveKey | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 03 E0 | ACE_Locking_GlobalRange_Set_RdLocke d | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 01 03 E0 | ACE_Locking_Range1_Set_RdLocked | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 08 00 00 00 ) 03 E0 00 (+NNNN | ACE_Locking_RangeNNNN_Set_RdLocked | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 03 E8 | ACE_Locking_GlobalRange_Set_WrLocke d | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 01 03 E8 | ACE_Locking_Range1_Set_WrLocked | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 08 00 00 00 ) 03 E8 00 (+NNNN | ACE_Locking_RangeNNNN_Set_WrLocked | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 01 03 F8 | ACE_MBRControl_Set_DoneToDOR | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 08 00 00 00 00 03 FC | ACE_DataStore_Get_All | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 08 00 00 00 01 03 FC | ACE_DataStore_Set_All | Set |  | ACE_ACE_Set_BooleanExpression |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Authority |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 00 00 00 | Authority | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 TT TT TT TT *AC7 | AuthorityObj | Get |  | ACE_Authority_Get_All, ACE_Anybody_Get_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 01 01 00 | Admin1 | Set |  | ACE_Admins_Set_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 09 00 00 00 02 01 00 | Admin2 | Set |  | ACE_Authority_Set_Enabled, ACE_Admins_Set_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 ) 01 00 00 (+XX XX | AdminXXXX | Set |  | ACE_Authority_Set_Enabled, ACE_Admins_Set_CommonNam e |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 01 03 00 | User1 | Set |  | ACE_Authority_Set_Enabled, ACE_User1_Set_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 09 00 00 00 ) 03 00 00 (+MMMM | UserMMMM | Set |  | ACE_Authority_Set_Enabled, ACE_User MMMM _Set_CommonNam e |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| C_PIN |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 00 00 00 | C_PIN | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 01 00 | C_PIN_Admin1 | Get |  | ACE_C_PIN_Admins_Get_All_NOPI N |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| <br>   <br>   <br>   <br> |  | 00 00 00 0B 00 ) 01 00 00 (+ XX XX | C_PIN_AdminXXXX | Get |  | ACE_C_PIN_Admins_Get_All_NOPI N |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 03 00 | C_PIN_User1 | Get |  | ACE_C_PIN_Admins_Get_All_NOPIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| <br>   <br>   <br>   <br>   <br> |  | 00 00 00 0B 00 ) 03 00 00 (+MM MM | C_PIN_UserMMMM | Get |  | ACE_C_PIN_Admins_Get_All_NOPI N |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 01 00 | C_PIN_Admin1 | Set |  | ACE_C_PIN_Admins_Set_PI N |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| <br>   <br>   <br>   <br> |  | 00 00 00 0B 00 ) 01 00 00 (+XX XX | C_PIN_AdminXXXX | Set |  | ACE_C_PIN_Admins_Set_PIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 0B 00 01 03 00 | C_PIN_User1 | Set |  | ACE_C_PIN_User1_Set_PIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| <br>   <br>   <br>   <br> |  | 00 00 00 0B 00 ) 03 00 00 (+MM MM | C_PIN_UserMMMM | Set |  | ACE_C_PIN_UserMMMM_Set_PIN |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| SecretProtect |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 00 00 1D 00 00 00 00 | SecretProtect | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 00 1D TT TT TT TT *AC8 | SecretProtectObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| LockingInfo |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 01 00 08 00 01 00 00 | LockingInfoObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Locking |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 02 00 08 00 00 00 00 | Locking | Next |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 02 00 08 00 01 00 00 | Locking_GlobalRange | Get |  | ACE_Locking_GlobalRange_Get_ RangeStartToActiveKey, ACE_Anybody_Get_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 02 00 08 00 01 03 00 | Locking_Range1 | Get |  | ACE_Locking_Range1_Get_ RangeStartToActiveKey, ACE_Anybody_Get_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| <br>   <br> |  | 00 02 00 08 00 ) 03 00 00 (+NN NN | Locking_RangeNNNN | Get |  | ACE_Locking_RangeNNNN_Get_ RangeStartToActiveKey, ACE_Anybody_Get_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 02 00 08 00 01 00 00 | Locking_GlobalRange | Set |  | ACE_Locking_GlblRng_Admins_Set, ACE_Locking_GlobalRange_Set_RdLocked, ACE_Locking_GlobalRange_Set_WrLocked, ACE_Admins_Set_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 02 00 08 00 01 03 00 | Locking_Range1 | Set |  | ACE_Locking_Admins_RangeStartToLOR, ACE_Locking_Range1_Set_RdLocked, ACE_Locking_Range1_Set_WrLocked, ACE_Admins_Set_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 00 08 02 00 ) 03 00 00 (+NN NN | Locking_RangeNNNN | Set |  | ACE_Locking_Admins_RangeStartToLOR, ACE_Locking_RangeNNNN_Set_RdLocked, ACE_Locking_RangeNNNN_Set_WrLocked, ACE_Admins_Set_CommonName |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| MBRControl |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 03 00 08 00 00 01 00 | MBRControlObj | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 03 00 08 00 01 00 00 | MBRControlObj | Set |  | ACE_MBRControl_Admins_Set, ACE_MBRControl_Set_DoneToDOR |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| MBR |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 04 00 08 00 00 00 00 | MBR | Get |  | ACE_Anybody |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 04 00 08 00 00 00 00 | MBR | Set |  | ACE_Admin |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| K_AES_128 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 05 00 08 00 01 00 00 | K_AES_128_GlobalRange_Key | Get |  | ACE_K_AES_Mode |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 05 00 08 00 01 03 00 | K_AES_128_Range1_Key | Get |  | ACE_K_AES_Mode |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 05 00 08 00 ) 03 00 00 (+NN NN | K_AES_128_RangeNNNN_Key | Get |  | ACE_K_AES_Mode |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 05 00 08 00 01 00 00 | K_AES_128_GlobalRange_Key | GenKey |  | ACE_K_AES_128_GlobalRange_GenKey |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 05 00 08 00 01 03 00 | K_AES_128_Range1_Key | GenKey |  | ACE_K_AES_128_Range1_GenKey |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 05 00 08 00 ) 03 00 00 (+NN NN | K_AES_128_RangeNNNN_Key | GenKey |  | ACE_K_AES_128_RangeNNNN_GenKey |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| K_AES_256 |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 06 00 08 00 01 00 00 | K_AES_256_GlobalRange_Key | Get |  | ACE_K_AES_Mode |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 06 00 08 00 01 03 00 | K_AES_256_Range1_Key | Get |  | ACE_K_AES_Mode |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 06 00 08 00 ) 03 00 00 (+NN NN | K_AES_256_RangeNNNN_Key | Get |  | ACE_K_AES_Mode |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 06 00 08 00 01 00 00 | K_AES_256_GlobalRange_Key | GenKey |  | ACE_K_AES_256_GlobalRange_GenKey |  |  |  | ACE_Anybody |  |  |  |  |  |  |

| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  |  | 00 06 00 08 00 01 03 00 | K_AES_256_Range1_Key | GenKey |  | ACE_K_AES_256_Range1_GenKey |  |  |  | ACE_Anybody |  |  |  |  |  |  |
|  |  | 00 06 00 08 00 ) 03 00 00 (+NN NN | K_AES_256_RangeNNNN_Key | GenKey |  | ACE_K_AES_256_RangeNNNN_GenKey |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| DataStore |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |
|  |  | 00 01 00 10 00 00 00 00 | DataStore | Get |  | ACE_DataStore_Get_All |  |  |  | ACE_Anybody |  |  |  |  |  |  |
| Tab le Association - informative only | UID | InvokingID | InvokingID Name - informative only | MethodID | CommonName | ACL | Log | AddACEACL | RemoveACEACL | GetACLACL | DeleteMethodACL | AddACELog | RemoveACELog | GetACLLog | DeleteMethodLog | LogTo |
|  |  | 00 01 00 10 00 00 00 00 | DataStore | Set |  | ACE_DataStore_Set_All |  |  |  | ACE_Anybody |  |  |  |  |  |  |

#### 4.3.1.7 ACE (M)

Table 39 contains Optional rows designated with (O).

*ACE1 means that the TPer SHALL support the values of “Admins” and “Admins OR UserMMMM” in the BooleanExpr column of each ACE_C_PIN_UserMMMM_Set_PIN ACE. The TPer SHALL fail the Set method invocation with status INVALID_PARAMETER if the host attempts to set a value not supported by the TPer.

| Table Asso ciation - Informative Column | UID | Name | CommonName | BooleanExpr | Columns |
| --- | --- | --- | --- | --- | --- |
| Base ACEs |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 00 00 01 | "ACE_Anybody" |  | Anybody | All |
|  | 00 00 00 08  <br> 00 00 00 02 | "ACE_Admin" |  | Admins | All |
|  | 00 00 00 08  <br> 00 00 00 03 | "ACE_Anybody_Get_CommonName" |  | Anybody | UID, CommonName |
|  | 00 00 00 08  <br> 00 00 00 04 | "ACE_Admins_Set_CommonName" |  | Admins | CommonName |
| ACE |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 80 00 | "ACE_ACE_Get_All" |  | Admins | All |
|  | 00 00 00 08  <br> 00 03 80 01 | "ACE_ACE_Set_BooleanExpression" |  | Admins | BooleanExpr |

**Table 39 - Locking SP - ACE Table Preconfiguration**

| Table Asso ciation - Informative Column | UID | Name | CommonName | BooleanExpr | Columns |
| --- | --- | --- | --- | --- | --- |
| Authority |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 90 00 | "ACE_Authority_Get_All" |  | Admins | All |
|  | 00 00 00 08  <br> 00 03 90 01 | "ACE_Authority_Set_Enabled" |  | Admins | Enabled |
|  | 00 00 00 08  <br> 00 04 40 01 | "ACE_User1_Set_CommonName" |  | Admins | CommonName |
|  | 00 00 00 08  <br> 00 04 40 00  <br> (+NN NN) | "ACE_UserMMMM_Set_CommonName" |  | Admins | CommonName |
| C_PIN |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 A0 00 | "ACE_C_PIN_Admins_Get_All_NOPIN" |  | Admins | UID, CharSet, TryLimit, Tries, Persistence |
|  | 00 00 00 08  <br> 00 03 A0 01 | "ACE_C_PIN_Admins_Set_PIN" |  | Admins | PIN |
|  | 00 00 00 08  <br> 00 03 A8 01 | "ACE_C_PIN_User1_Set_PIN" |  | Admins OR User1  <br> *ACE1 | PIN |
| (O) | 00 00 00 08  <br> 00 03 A8 00   <br> (+MMMM) | "ACE_C_PIN_UserMMMM_Set_PIN" |  | Admins OR  <br> UserMMMM  <br> *ACE1 | PIN |
| K_AES |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 BF FF | "ACE_K_AES_Mode" |  | Anybody | Mode |
| K_AES_128 |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 B0 00 | "ACE_K_AES_128_GlobalRange_  <br> GenKey" |  | Admins | All |
|  | 00 00 00 08  <br> 00 03 B0 01 | "ACE_K_AES_128_Range1_ GenKey" |  | Admins | All |

| Table Asso ciation - Informative Column | UID | Name | CommonName | BooleanExpr | Columns |
| --- | --- | --- | --- | --- | --- |
| (O) | 00 00 00 08  <br> 00 03 B0 00  <br> (+NNNN) | "ACE_K_AES_128_RangeNNNN_  <br> GenKey" |  | Admins | All |
| K_AES_256 |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 B8 00 | "ACE_K_AES_256_GlobalRange_  <br> GenKey" |  | Admins | All |
|  | 00 00 00 08  <br> 00 03 B8 01 | "ACE_K_AES_256_Range1_ GenKey" |  | Admins | All |
|  | 00 00 00 08  <br> 00 03 B8 00  <br> (+NNNN) | "ACE_K_AES_256_RangeNNNN_  <br> GenKey" |  | Admins | All |
| Locking |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 D0 00 | "ACE_Locking_GlobalRange_Get_ RangeStartToActiveKey" |  | Admins | RangeStart,  <br> RangeLength,  <br> ReadLockEnabled,  <br> WriteLockEnabled,  <br> ReadLocked,   <br> WriteLocked,  <br> LockOnReset, ActiveKey |
|  | 00 00 00 08  <br> 00 03 D0 01 | "ACE_Locking_Range1_Get_ RangeStartToActiveKey" |  | Admins | RangeStart,  <br> RangeLength,  <br> ReadLockEnabled,  <br> WriteLockEnabled,  <br> ReadLocked,   <br> WriteLocked,  <br> LockOnReset, ActiveKey |
|  | 00 00 00 08  <br> 00 03 D0 00  <br> (+NNNN) | "ACE_Locking_RangeNNNN_Get_ RangeStartToActiveKey" |  | Admins | RangeStart,  <br> RangeLength,  <br> ReadLockEnabled,  <br> WriteLockEnabled,  <br> ReadLocked,   <br> WriteLocked,  <br> LockOnReset, ActiveKey |

| Table Asso ciation - Informative Column | UID | Name | CommonName | BooleanExpr | Columns |
| --- | --- | --- | --- | --- | --- |
|  | 00 00 00 08  <br> 00 03 E0 00 | "ACE_Locking_GlobalRange_Set_RdLocked" |  | Admins | ReadLocked |
|  | 00 00 00 08  <br> 00 03 E0 01 | "ACE_Locking_Range1_Set_RdLocked" |  | Admins | ReadLocked |
|  | 00 00 00 08  <br> 00 03 E0 00  <br> (+NNNN) | "ACE_Locking_RangeNNNN_Set_RdLocked" |  | Admins | ReadLocked |
|  | 00 00 00 08  <br> 00 03 E8 00 | "ACE_Locking_GlobalRange_Set_WrLocked" |  | Admins | WriteLocked |
|  | 00 00 00 08  <br> 00 03 E8 01 | "ACE_Locking_Range1_Set_WrLocked" |  | Admins | WriteLocked |
|  | 00 00 00 08  <br> 00 03 E8 00  <br> (+NNNN) | "ACE_Locking_RangeNNNN_Set_WrLocked" |  | Admins | WriteLocked |
|  | 00 00 00 08  <br> 00 03 F0 00 | "ACE_Locking_GlblRng_Admins_Set" |  | Admins | ReadLockEnabled,  <br> WriteLockEnabled,  <br> ReadLocked,  <br> WriteLocked,  <br> LockOnReset |
|  | 00 00 00 08  <br> 00 03 F0 01 | "ACE_Locking_Admins_RangeStartToLOR" |  | Admins | RangeStart,  <br> RangeLength,  <br> ReadLockEnabled,  <br> WriteLockEnabled,  <br> ReadLocked,  <br> WriteLocked,  <br> LockOnReset |
| MBRControl |  |  |  |  |  |
|  | 00 00 00 08  <br> 00 03 F8 00 | "ACE_MBRControl_Admins_Set" |  | Admins | Enable, Done, DoneOnReset |
|  | 00 00 00 08  <br> 00 03 F8 01 | "ACE_MBRControl_Set_DoneToDOR" |  | Admins | Done, DoneOnReset |
| DataStore |  |  |  |  |  |
| Table Asso ciation - Informative Column | UID | Name | CommonName | BooleanExpr | Columns |
|  | 00 00 00 08  <br> 00 03 FC 00 | "ACE_DataStore_Get_All" |  | Admins | All |
|  | 00 00 00 08  <br> 00 03 FC 01 | "ACE_DataStore_Set_All" |  | Admins | All |

#### 4.3.1.8 Authority (M)

Table 40 contains Optional rows designated with (O). Notes:

1. Admin1 is required; Admin2 to Admin4 are required but disabled in OFS state.

Any additional Admin authorities are (O).

2. User1 through User8 SHALL be implemented.

**Table 40 - Locking SP - Authority Table Preconfiguration**

| UID | Name | CommonName | IsClass | Class | Enabled | Secure | HashAndSign | PresentCertificate | Operation | Credential | ResponseSign | ResponseExch | ClockStart | ClockEnd | Limit | Uses | Log | LogTo |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 09  <br> 00 00 00 01 | "Anybody" | "" | F | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 00 00 02 | "Admins" | "" | T | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 01 00 01 | "Admin1" | "" | F | Admins | T | None | None | F | Password | C_PIN_Admin1 | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 01 00 02 | "Admin2" | "" | F | Admins | F | None | None | F | Password | C_PIN_Admin2 | Null | Null |  |  |  |  |  |  |
| UID | Name | CommonName | IsClass | Class | Enabled | Secure | HashAndSign | PresentCertificate | Operation | Credential | ResponseSign | ResponseExch | ClockStart | ClockEnd | Limit | Uses | Log | LogTo |
| 00 00 00 09  <br> 00 01 00 03 | "Admin3" | "" | F | Admins | F | None | None | F | Password | C_PIN_Admin3 | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 01 00 04 | "Admin4" | "" | F | Admins | F | None | None | F | Password | C_PIN_Admin4 | Null | Null |  |  |  |  |  |  |
| 00 00 00 09 00 01 00 00  <br> (+XX XX)1  <br> (O) | "AdminXXXX" | "" | F | Admins | F |  |  |  |  |  |  |  |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 03 00 00 | "Users" | "" | T | Null | T | None | None | F | None | Null | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 03 00 01 | "User1" | "" | F | Users | F | None | None | F | Password | C_PIN_User1 | Null | Null |  |  |  |  |  |  |
| 00 00 00 09  <br> 00 03 00 00  <br> (+MM MM)2  <br> (O) | "UserMMMM" | "" | F | Users | F | None | None | F | Password | C_PIN_UserMMMM | Null | Null |  |  |  |  |  |  |

#### 4.3.1.9 C_PIN (M)

Table 41 includes Optional rows designated with (O) Notes:

1. If the Locking SP's original life cycle state is Manufactured-Inactive, see 5.1.1.2 for the initial value of C_PIN_Admin1.PIN. If the Locking SP's original life cycle state is Manufactured, then the initial value of C_PIN_Admin1.PIN is the same as the Admin SP's C_PIN_MSID.PIN value.

**Table 41 - Locking SP - C_PIN Table Preconfiguration**

| UID | Name | CommonName | PIN | CharSet | TryLimit | Tries | Persistence |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 00 0B 00 01 00 01 | "C_PIN_Admin1" |  | SID or  <br> MSID1 | Null | 0 | 0 | FALSE |
| 00 00 00 0B 00 01 00 02 | "C_PIN_Admin2" |  | “” | Null | 0 | 0 | FALSE |
| 00 00 00 0B 00 01 00 03 | "C_PIN_Admin3" |  | “” | Null | 0 | 0 | FALSE |
| 00 00 00 0B 00 01 00 04 | "C_PIN_Admin4" |  | “” | Null | 0 | 0 | FALSE |
| 00 00 00 0B  <br> 00 01 00 00  <br> (+XX XX)  <br> (O) | "C_PIN_AdminXXXX" |  | “” | Null | 0 | 0 | FALSE |
| 00 00 00 0B 00 03 00 01 | "C_PIN_User1" |  | “” | Null | 0 | 0 | FALSE |
| 00 00 00 0B  <br> 00 03 00 00   <br> (+MM MM)  <br> (O) | "C_PIN_UserMMMM" |  | “” | Null | 0 | 0 | FALSE |

#### 4.3.1.10 SecretProtect (M)

At least one of the objects shown in Table 42 SHALL be supported

**Table 42 - Locking SP - SecretProtect Table Preconfiguration**

| UID | Table | ColumnNumber | ProtectMechanisms |
| --- | --- | --- | --- |
| 00 00 00 1D  <br> 00 00 00 1D | 00 00 00 01  <br> 00 00 08 05  <br> (K_AES_128) | 0x03 | VU |
| 00 00 00 1D 00 00 00 1E | 00 00 00 01  <br> 00 00 08 06  <br> (K_AES_256) | 0x03 | VU |

Note: The “VU” entries in Table 42 indicate that this specification does not require a specific value to be reported in the ProtectMechanisms cell. It is NOT a requirement to report the “Vendor Unique” protect_types value (Refer to [2] for details).

### 4.3.2 Base Template Methods

Refer to section 4.3.1.5 for supported methods.

### 4.3.3 Crypto Template Tables

An Opal SSC compliant Storage Device is not required to support any Crypto template tables.

### 4.3.4 Crypto Template Methods

Refer to section 4.3.1.5 for supported methods.

#### 4.3.4.1 Random

Refer to section 4.2.9.1 for additional constraints imposed on the Random method.

### 4.3.5 Locking Template Tables

#### 4.3.5.1 LockingInfo (M)

The LockingInfo table has the columns defined in Table 43, in addition to those defined in [2]:

**Table 43 - Locking SP – LockingInfo Columns**

| Column Number | Column Name | IsUnique | Column Type |
| --- | --- | --- | --- |
| 0x07 | AlignmentRequired |  | boolean |
| 0x08 | LogicalBlockSize |  | uinteger_4 |
| 0x09 | AlignmentGranularity |  | uinteger_8 |
| 0x0A | LowestAlignedLBA |  | uniteger_8 |

- **AlignmentRequired**

This column indicates whether the TPer requires ranges in the Locking table to be aligned (see section

4.3.5.2.1). If AlignmentRequired is TRUE, then the TPer requires ranges to be aligned. If AlignmentRequired is FALSE, then the TPer does not require ranges to be aligned.

This column SHALL NOT be modifiable by the host and MAY be retrieved by Anybody. • **LogicalBlockSize**

This column indicates the number of bytes in a logical block.

This column SHALL NOT be modifiable by the host and MAY be retrieved by Anybody. • **AlignmentGranularity**

This column indicates the number of logical blocks in a group, for alignment purposes (see section 5.4). This column SHALL NOT be modifiable by the host and MAY be retrieved by Anybody.

- **LowestAlignedLBA**

This column indicates the lowest logical block address that is located at the beginning of an alignment granularity group (see section 5.4).

This column SHALL NOT be modifiable by the host and MAY be retrieved by Anybody.

**Table 44 - Locking SP - LockingInfo Table Preconfiguration**

| UID | Name | Version | EncryptSupport | MaxRanges | MaxReEncryptions | KeysAvailableCfg | AlignmentRequired | LogicalBlockSize | AlignmentGranularity | LowestAlignedLBA |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00 08 01 00 00 00 01 |  |  | Media Encryption | 81 |  |  |  |  |  |  |

Note:

1. The MaxRanges column in Table 44 specifies the number of supported ranges and SHALL have a minimum of 8 ranges.

#### 4.3.5.2 Locking (M)

Table 45 contains Optional rows designated with (O).

*LT1 means that the ActiveKey can be a K_AES_128 object reference (UID) or a K_AES_256 object reference (UID) *LT2 means that only a limited set of LockOnReset values is required to be supported by Opal SSC compliant Storage Devices. Refer to section 4.3.5.2.2 for details.

**Table 45 - Locking SP - Locking Table Preconfiguration**

| UID | Name | CommonName | Rang eStart | RangeLength | ReadLockEnabled | WriteLockEnabled | ReadLocked | WriteLocked | LockOnReset | ActiveKey | NextKey | ReEncryptState | ReEncyptRequest | AdvKeyMode | VerifyMode | ContOnReset | LastReEncryptLBA | LastReEncState | GeneralStatus |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 00 00  <br> 08 02  <br> 00 00 00 01 | " Locking_GlobalRange" | "" | 0 | 0 | F | F | F | F | Power Cycle *LT2 | K_AES_128[256]_GlobalRange_Key * LT1 |  |  |  |  |  |  |  |  |  |
| 00 00  <br> 08 02  <br> 00 03 00 01 | "Locking_Range1" | "" | 0 | 0 | F | F | F | F | Power Cycle *LT2 | K_AES_128[256]_Range1_Key *LT 1 |  |  |  |  |  |  |  |  |  |
| UID | Name | CommonName | Rang eStart | RangeLength | ReadLockEnabled | WriteLockEnabled | ReadLocked | WriteLocked | LockOnReset | ActiveKey | NextKey | ReEncryptState | ReEncyptRequest | AdvKeyMode | VerifyMode | ContOnReset | LastReEncryptLBA | LastReEncState | GeneralStatus |
| 00 00  <br> 08 02  <br> 00 03  <br> NN NN | " Locking_RangeNNNN" | "" | 0 | 0 | F | F | F | F | Power Cycle *LT2 | K_AES_128[256]_RangeNNNN_Key *LT 1 |  |  |  |  |  |  |  |  |  |

##### 4.3.5.2.1 Geometry Reporting Feature Behavior

The following behaviors SHALL be implemented.

**4.3.5.2.1.1 RangeStart Behavior**

This column value defines the starting LBA value for this range. In non-Global Range rows, this column MAY be modifiable based on access control settings. Changes to this column are subject to the same constraints and checks defined for this column when rows of the Locking table are created (see [2]).

When processing a Set method or CreateRow method on the Locking table for a non-Global Range row, if:

1. the AlignmentRequired column in the LockingInfo table is TRUE;
2. RangeStart is non-zero; and
3. StartAlignment (see Figure 1) is non-zero,

then the method SHALL fail and return an error status code INVALID_PARAMETER. **Figure 1 - StartAlignment Calculation**

StartAlignment = (RangeStart - LowestAlignedLBA) modulo AlignmentGranularity where:

LowestAlignedLBA and AlignmentGranularity are columns in the LockingInfo table (see section 4.3.5.1)

**4.3.5.2.1.2 RangeLength Behavior**

This column value defines the quantity of contiguous LBAs for this LBA range (starting with the value defined in the RangeStart column). In non-Global Range rows, this column MAY be modifiable based on access control settings. Changes to this column are subject to the same constraints and checks defined for this column when rows of the Locking table are created (see [2]).

When processing a Set method or CreateRow method on the Locking table for a non-Global Range row, if:

1. the AlignmentRequired column in the LockingInfo table is TRUE;
2. RangeLength is non-zero; and
3. LengthAlignment (see Figure 2) is non-zero,

then the method SHALL fail and return an error status code INVALID_PARAMETER. **Figure 2 - LengthAlignment Calculation**

| <br>     If RangeStart is zero, then  <br>            LengthAlignment = (RangeLength - LowestAlignedLBA) modulo AlignmentGranularity   <br>     If RangeStart is non-zero, then  <br>            LengthAlignment = (RangeLength modulo AlignmentGranularity)  where:  <br> LowestAlignedLBA and AlignmentGranularity are columns in the LockingInfo table (see section 4.3.5.1)       <br> |
| --- |

##### 4.3.5.2.2 LockOnReset Restrictions

The TPer SHALL support the following LockOnReset column values:

1. { 0 } (i.e. Power Cycle); and
2. { 0, 3 } (i.e. Power Cycle and Programmatic).

Additionally, the TPer MAY support the following LockOnReset column values:

1. { 0, 1 } (i.e. Power Cycle and Hardware Reset); and
2. { 0,1, 3 } (i.e. Power Cycle, Hardware Reset and Programmatic).

#### 4.3.5.3 MBRControl (M)

The MBRControl table is defined in [2], and Table 46 defines the Preconfiguration Data for the MBRControl table.

*MC1 means that only a limited set of DoneOnReset values is required to be supported by Opal SSC compliant Storage Devices. Refer to section 4.3.5.3.1 for details.

**Table 46 - Locking SP - MBRControl Table Preconfiguration**

| UID | Enable | Done | DoneOnReset |
| --- | --- | --- | --- |
| 00 00 08 03 00 00 00 01 | False | False | <br> Power Cycle  <br> *MC1  <br> |

##### 4.3.5.3.1 DoneOnReset Restrictions

The TPer SHALL support the following DoneOnReset column values:

1. { 0 } (i.e. Power Cycle); and
2. { 0, 3 } (i.e. Power Cycle and Programmatic).

Additionally, the TPer MAY support the following DoneOnReset column values:

1. { 0, 1 } (i.e. Power Cycle and Hardware Reset); and
2. { 0,1, 3 } (i.e. Power Cycle, Hardware Reset and Programmatic).

#### 4.3.5.4 MBR (M)

The MBR minimum size SHALL be 128 MB (0x08000000).

The initial contents of the MBR table SHALL be vendor unique.

#### 4.3.5.5 K_AES_128 or K_AES_256 (M)

At least one of the following tables (Table 47 and Table 48) SHALL be supported. Table 47 contains Optional rows designated with (O).

*K1 means that a field is indirectly writable using the GenKey method.

**Table 47 - Locking SP - K_AES_128 Table Preconfiguration**

| UID | Name | CommonName | Key | Mode |
| --- | --- | --- | --- | --- |
| 00 00 08 05 00 00 00 01 | "K_AES_128_GlobalRange_Key" |  | VU  *K1 | VU |
| 00 00 08 05 00 03 00 01 | "K_AES_128_Range1_Key" |  | VU  *K1 | VU |
| 00 00 08 05 00 03 NN NN  <br> (O) | "K_AES_128_RangeNNNN_Key" |  | VU *K1 | VU |

**Table 48 - Locking SP - K_AES_256 Table Preconfiguration**

| UID | Name | CommonName | Key | Mode |
| --- | --- | --- | --- | --- |
| 00 00 08 0600 00 00 01 | "K_AES_256_GlobalRange_Key" |  | VU *K1 | VU |
| 00 00 08 06  <br> 00 03 00 01 | "K_AES_256_Range1_Key" |  | VU *K1 | VU |
| 00 00 08  06  <br> 00 03 NN NN  <br> (O) | "K_AES_256_RangeNNNN_Key" |  | VU *K1 | VU |

### 4.3.6 Locking Template Methods

Refer to Section 4.3.1.5 for supported methods.

### 4.3.7 Storage Device Read/Write Data Command Locking Behavior Interactions with Range Crossing

If a Storage Device receives a read or write command that spans multiple Locking ranges and the Locking ranges are not locked, the Storage Device SHALL either:

- Process the data transfer as defined in [2], if Range Crossing Behavior bit is set to zero (in Level 0 Discovery Opal SSC V2 Feature, see section 3.1.1.5) OR
- Terminate the command with “Other Invalid Command Parameter” as defined in [4], if Range Crossing Behavior bit is set to one (in Level 0 Discovery Opal SSC V2 Feature, see section 3.1.1.5).

### 4.3.8 Non Template Tables

#### 4.3.8.1 DataStore (M)

The DataStore is a byte table. It can be used by the host for generic secure data storage. The DataStore table SHALL be at least 10MB in size (the Table table object that represents the DataStore table SHALL have a Rows column value of at least 0x00A00000). The access control for modification or retrieval of data in the table initially requires a member of the Admins class authority. These access control settings are personalizable. The Initial DataStore content value is VU.

# 5 Appendix – SSC Specific Features

## 5.1 Opal SSC-Specific Methods

### 5.1.1 Activate – Admin Template SP Object Method

Activate is an Opal SSC-specific method for managing the life cycle of SPs created in manufacturing (Manufactured

SP), whose initial life cycle state is “Manufactured-Inactive”. The following pseudo-code is the signature of the Activate method (see [2] for more information).

SPObjectUID.Activate[ ]

=>

[ ]

Activate is an object method that operates on objects in the Admin SP’s SP table. The TPer SHALL NOT permit Activate to be invoked on the SP objects of issued SPs.

Invocation of Activate on an SP object that is in the “Manufactured-Inactive” state causes the SP to transition to the “Manufactured” state. Invocation of Activate on an SP in any other life cycle state SHALL complete successfully provided access control is satisfied, and have no effect. The Activate method allows the TPer owner to “turn on” an SP that was created in manufacturing.

This method operates within a Read-Write session to the Admin SP. The SP SHALL be activated immediately after the method returns success if its invocation is not contained within a transaction.

In case of an “Activate Error” (see [4]), Activate SHALL fail with a status of FAIL.

The MethodID for Activate SHALL be 0x00 00 00 06 00 00 02 03.

#### 5.1.1.1 Activate Support

Support for Activate within transactions is (N), and the behavior of Activate within transactions is out of the scope of this specification.

If the Locking SP was created in manufacturing, and its Original Factory State is Manufactured-Inactive (see section 5.2.2), support for Activate on the Locking SP’s object in the SP table is Mandatory.

#### 5.1.1.2 Side effects of Activate

Upon successful activation of an SP that was in the “Manufactured-Inactive” state, the following changes SHALL be made:

- The LifeCycleState column of SP’s object in the Admin SP’s SP table SHALL change to “Manufactured”.
- The current SID PIN (C_PIN_SID) in the Admin SP is copied into the PIN column of Admin1’s C_PIN credential (C_PIN_Admin1) in the activated SP. This allows for taking ownership of the SP with a known PIN credential.
- Any TPer functionality affected by the life cycle state of the SP based on the SP’s templates is modified as defined in the appropriate Template reference section of [2], and as defined in the “State transitions for Manufactured SPs” section (see section 5.2.2.2) and “State behaviors for Manufactured SPs” section (see section 5.2.2.3) of this specification.

### 5.1.2 Revert – Admin Template SP Object Method

Revert is an Opal SSC-specific method for managing the life cycle of SPs created in manufacturing (Manufactured SP). The following pseudo-code is the signature of the Revert method (see [2] for more information).

SPObjectUID.Revert[ ]

=>

[ ]

Revert is an object method that operates on objects in the Admin SP’s SP table. The TPer SHALL NOT permit Revert to be invoked on the SP objects of issued SPs.

Invoking Revert on an SP object causes the SP to revert to its Original Factory State. This method allows the TPer owner (or TPer manufacturer, if access control permits and the Maker authorities are enabled) to remove the SP owner’s ownership of the SP and revert the SP to its Original Factory State.

Invocation of Revert is permitted on Manufactured SPs that are in any life cycle state. Successful invocation of Revert on a Manufactured SP that is in the Manufactured-Inactive life cycle state SHALL have no effect on the SP.

This method operates within a Read-Write session to the Admin SP. The TPer SHALL revert the SP immediately after the method is successfully invoked outside of a transaction. If Revert is invoked on the Admin SP’s object in the SP table, the TPer SHALL abort the session immediately after reporting status of the method invocation if invoked outside of a transaction. The TPer MAY prepare a CloseSession method for retrieval by the host to indicate that the session has been aborted.

The MethodID for Revert SHALL be 0x00 00 00 06 00 00 02 02.

#### 5.1.2.1 Revert Support

Support for Revert within transactions is (N), and the behavior of Revert within transactions is out of the scope of this specification.

Support for Revert on the Admin SP’s object in the SP table is Mandatory. (Note that the OFS of the Admin SP is Manufactured, see section 5.2.2).

If the Locking SP was created in manufacturing, support for Revert on the Locking SP’s object in the SP table is Mandatory.

#### 5.1.2.2 Effects of Revert

Upon successful invocation of the Revert method, the following changes SHALL be made:

- If the Locking SP is not in the “Manufactured-Inactive” life cycle state, then successful invocation of the Revert method on the Locking SP or Admin SP SHALL cause user data removal as defined by the ActiveDataRemovalMechanism (see Table 33) and cause the media encryption keys to be eradicated, which has the side effect of securely erasing all data in the User LBA portion of the Storage Device.
- If the Locking SP is in the “Manufactured-Inactive” life cycle state, then successful invocation of the Revert method on the Locking SP SHALL NOT cause user data removal in the Storage Device.

Interactions with interface commands during the processing of the Revert method are defined in [4].

If any TCG reset occurs prior to completing user data removal and the eradication of all media encryption keys in the Storage Device, then the Revert operation SHALL be aborted and the Locking SP SHALL NOT revert to its Original Factory State.

**Start of Informative Comment**

If any TCG reset occurs during the processing of the Revert method, the result of user data removal is undefined and the TPer does not erase personalization of the Locking SP. For example, the PIN column value for each row in C_PIN table is unchanged.

**End of Informative Comment**

Upon completion of user data removal and the eradication of all media encryption keys in the Storage Device, or if the Locking SP is in the “Manufactured-Inactive” life cycle state, the following changes SHALL be made:

- The row in the Admin SP’s SP table that represents the invoked SP SHALL revert to its original factory values.
- The SP itself SHALL revert to its Original Factory State. While reverting to its Original Factory State, the TPer SHALL securely erase all personalization of the SP, and return personalized values to their Original Factory State values. The mechanism for erasure of personalization is implementation-specific.
- When the Revert method is successfully invoked on the SP object for the Admin SP (UID = 0x00 00 02 05 00 00 00 01), the entire TPer SHALL revert to its Original Factory State, including:
  - All Admin SP personalization with the exception of the PIN column value of the C_PIN_SID object. See section 5.1.2.2.1 for the effects of the Revert method upon the PIN column value of the C_PIN_SID object.
  - All issued SPs SHALL be deleted, and all Manufactured SPs SHALL revert to Original Factory State. Manufactured SPs in the “Manufactured-Inactive” life cycle state SHALL NOT be affected.
- Any TPer functionality affected by the life cycle state of the SP based on the templates incorporated into it is modified as defined in the appropriate Template reference section of [2], and as defined in the “State transitions for Manufactured SPs” section (see section 5.2.2.2) and “State behaviors for Manufactured SPs” section (see section 5.2.2.3) of this specification.

**Start of Informative Comment**

Unless already in the Manufactured-Inactive life cycle state, reverting the Locking SP will cause the media encryption keys to be eradicated, which has the side effect of securely erasing all data in the User LBA portion of the Storage Device.

**End of Informative Comment**

##### 5.1.2.2.1 Effects of Revert on the PIN Column Value of C_PIN_SID

When Revert is successfully invoked on the SP object for the Admin SP (UID = 0x00 00 02 05 00 00 00 01), the PIN column value of the C_PIN_SID object SHALL be affected as follows:

1. If the SID authority has never been successfully authenticated, then the C_PIN_SID PIN column SHALL remain at its current value.
2. If the SID authority has previously been successfully authenticated, then:
  1. If the value of the “Behavior of C_PIN_SID PIN upon TPer Revert” field in the Opal SSC V2 feature descriptor is 0x00, then the C_PIN_SID PIN column SHALL be set to the PIN column value of the C_PIN_MSID object. Additionally, the “Initial C_PIN_SID PIN Indicator” field SHALL be set to 0x00 upon completion of the Revert.
  2. If the value of the “Behavior of C_PIN_SID PIN upon TPer Revert” field in the Opal SSC V2 feature descriptor is not 0x00, then the C_PIN_SID PIN column SHALL be set to a vendor unique (VU) value.

**Start of Informative Comment**

In the case where the “Initial C_PIN_SID PIN Indicator” and “Behavior of C_PIN_SID PIN upon TPer Revert” fields are both 0x00, the above rules for Revert are backward compatible with Opal v1.00.

**End of Informative Comment**

#### 5.1.2.3 Interrupted Revert

The Revert method and complete implementation of necessary background operations MAY be aborted due to any reset condition, including power loss.

When interrupted, the Data Removal Operation Interrupted bit SHALL be set to one in the Level 0 Discovery – Supported Data Removal Mechanism feature descriptor appropriately as defined in section 3.1.1.6.2.

Further, the return status value of the Revert method does not mean that all necessary operations, such as the background deallocate, or trim, or un-map are complete.

### 5.1.3 RevertSP – Base Template SP Method

RevertSP is an Opal SSC-specific method for managing the life cycle of an SP, if it was created in manufacturing (Manufactured SP). The following pseudo-code is the signature of the RevertSP method (see [2] for more information).

ThisSP.RevertSP[ KeepGlobalRangeKey = boolean ]

=>

[ ]

RevertSP is an SP method in the Base Template.

Invoking RevertSP method on an SP SHALL cause it to revert to its Original Factory State. This method allows the SP owner to relinquish control of the SP and revert the SP to its Original Factory State.

This method operates within a Read-Write session to an SP. The TPer SHALL revert the SP immediately after the method is successfully invoked outside of a transaction. Upon completion of reverting the SP, the TPer SHALL report status of the method invocation if invoked outside of a transaction, and then immediately abort the session. The TPer MAY prepare a CloseSession method for retrieval by the host to indicate that the session has been aborted. The MethodID for RevertSP SHALL be 0x00 00 00 06 00 00 00 11.

#### 5.1.3.1 RevertSP Support

Support for RevertSP within transactions is (N), and the behavior is out of the scope of this document.

If the Locking SP was created in manufacturing, support for RevertSP on the Locking SP is Mandatory.

#### 5.1.3.2 KeepGlobalRangeKey parameter (Locking Template-specific)

The Optional **KeepGlobalRangeKey** parameter is a Locking Template-specific parameter. This parameter provides a mechanism for the Locking SP to be “turned off” without eradicating the media encryption key for the Global Locking Range. This allows the Locking SP to be disabled without causing removal of the user data associated with the Global Locking Range.

When this parameter is present and set to True, the TPer SHALL NOT erase data associated with the Global Locking Range after the Locking SP transitions to the “Manufactured-Inactive” state even if the valid value is set to the ActiveDataRemovalMechanism parameter in DataRemovalMechanism table.

If the Global Range is either Read Unlocked or Write Unlocked at the time of invocation of RevertSP, then the TPer SHALL comply with the request to keep the user data associated with the Global locking range and the Global Range’s media encryption key.

If the Global Range is Read Locked and Write Locked then invocation of the RevertSP method with the

**KeepGlobalRangeKey** parameter set to True SHALL fail with status FAIL, and the SP SHALL NOT change life cycle states.

If the Locking SP was created in manufacturing, support for the **KeepGlobalRangeKey** parameter is Mandatory for the Locking SP.

The parameter number for **KeepGlobalRangeKey** SHALL be 0x060000.

#### 5.1.3.3 Effects of RevertSP

Upon successful invocation of the RevertSP method, the following changes SHALL be made:

- If the **KeepGlobalRangeKey** parameter is not present or set to False, then successful invocation of the RevertSP method on the Locking SP or Admin SP SHALL cause user data removal as defined by the ActiveDataRemovalMechanism (see Table 33) and cause the media encryption keys to be eradicated, which has the side effect of securely erasing all data in the User LBA portion of the Storage Device.
- If the **KeepGlobalRangeKey** parameter is set to True, then successful invocation of the RevertSP method on the Locking SP SHALL cause user data removal as defined by the ActiveDataRemovalMechanism (see Table 33) and cause all media encryption keys to be eradicated except for the Global Range’s media encryption key (K_AES_{128,256}_GlobalRange_Key).
- Interactions with interface commands during the processing of the RevertSP method are defined in [4].

If any TCG reset occurs prior to completing user data removal and the eradication of media encryption keys in the Storage Device, then the operation SHALL be aborted and the Locking SP SHALL NOT revert to its Original Factory State.

**Start of Informative Comment**

If any TCG reset occurs during the processing of the RevertSP method, the result of user data removal is undefined. **End of Informative Comment**

Upon completion of user data removal and the eradication of media encryption keys in the Storage Device, the following changes SHALL be made:

- The row in the Admin SP’s SP table that represents the Locking SP SHALL revert to its original factory value. • The Locking SP itself SHALL revert to its Original Factory State. While reverting to its Original Factory State, the TPer SHALL erase all personalization of the SP, and return the personalized values to their Original Factory State values. The mechanism for erasure of personalization implementation-specific.
- Any TPer functionality affected by the life cycle state of the SP based on the templates incorporated into it is modified as defined in the appropriate Template reference section of [2], and as defined in the “State transitions for Manufactured SPs” section (see section 5.2.2.2) and “State behaviors for Manufactured SPs” section (see section 5.2.2.3) of this specification.

**Start of Informative Comment**

Reverting the Locking SP will cause the media encryption keys to be eradicated (except for the GlobalRange key if the **KeepGlobalRangeKey** parameter is present and set to True), which has the side effect of securely erasing all data in the User LBA portion of the Storage Device.

**End of Informative Comment**

#### 5.1.3.4 Interrupted RevertSP

The RevertSP method and complete implementation of the necessary background operations MAY be aborted due to any reset condition, including power loss.

When interrupted, the Data Removal Operation Interrupted bit SHALL be set to one in the Level 0 Discovery – Supported Data Removal Mechanism feature descriptor appropriately as defined in section 3.1.1.6.2.

Further, the return status value of the RevertSP method does not mean that all necessary operations such as the data removal operation are complete.

## 5.2 Life Cycle

### 5.2.1 Issued vs. Manufactured SPs

#### 5.2.1.1 Issued SPs

For Opal SSC-compliant TPers that support issuance, refer to [2] for the life cycle states and life cycle management.

#### 5.2.1.2 Manufactured SPs

Opal SSC-compliant SPs that are created in manufacturing (Manufactured SPs) SHALL NOT have an implementationspecific life cycle, and SHALL conform to the life cycle defined in section 5.2.2.

### 5.2.2 Manufactured SP Life Cycle States

The state diagram for Manufactured SPs is shown in Figure 3.

**Figure 3 - Life Cycle State Diagram for Manufactured SPs**

**Start of Informative Comment**

The LockingSP’s OFS could be either the Manufactured-Inactive or the Manufactured state. The Manufactured-toManufactured optional state transition may occur when the LockingSP’s OFS is the Manufactured state. In an implementation where the LockingSP’s OFS is the Manufactured state, then the Revert method would revert the LockingSP from Manufactured back to Manufactured while still reverting all LockingSP tables back to the OFS. **End of Informative Comment**

Additional state transitions may exist depending on the states supported by the Storage Device and the SP’s Original Factory State. Invoking Revert or RevertSP (see sections 5.1.2 and 5.1.2.3) on the SP will cause the SP to transition back to its Original Factory State.

The Original Factory State of the Admin SP SHALL be Manufactured. The only state that is Mandatory for the Admin SP is Manufactured.

If the Locking SP is a Manufactured SP, then its Original Factory State SHALL be Manufactured-Inactive.

If the Locking SP is a Manufactured SP, then support for the states of Manufactured and Manufactured-Inactive are mandatory.

The other states in the state diagram are beyond the scope of this document.

#### 5.2.2.1 State definitions for Manufactured SPs

1. **Manufactured-Inactive**: This is the Original Factory State for SPs that are created in manufacturing, where it is not desired for the functionality of that SP to be active when the TPer is shipped. All templates that exist in an SP that is in the Manufactured-Inactive state SHALL be counted in the Instances column of the appropriate objects in the Admin SP’s Template table. Sessions cannot be opened to SPs in the Manufactured-Inactive state. Only SPs whose Original Factory State was Manufactured-Inactive can return to the Manufactured-Inactive state.
2. **Manufactured**: This is the standard operational state of a Manufactured SP, and defines the initial required access control settings of an SP based on the Templates incorporated into the SP, prior to personalization.

The Manufactured state is Mandatory for the Admin SP.

#### 5.2.2.2 State transitions for Manufactured SPs

The following sections describe the Mandatory and Optional state transitions for Opal SSC-compliant Manufactured SPs.

For the Admin SP, the only transition for which support is mandatory is “ANY STATE to ORIGINAL FACTORY STATE” (see section 5.2.2.2.2). As the only mandatory state for the Admin SP is Manufactured, the only mandatory transition is from Manufactured to Manufactured with the side effect of reverting the entire TPer to its Original Factory State. See section 5.1.2 for details.

If the Locking SP is a Manufactured SP, support for the “ANY STATE to ORIGINAL FACTORY STATE” transition (see section 5.2.2.2.2) is Mandatory. Specifically, support for the transition from Manufactured to either ManufacturedInactive or Manufactured is Mandatory, depending on the Locking SP’s Original Factory State. This transition is accomplished via the Revert or RevertSP method (see sections 5.1.2 and 5.1.2.3).

If the Locking SP’s Original Factory State is Manufactured-Inactive, then support for the “Manufactured-Inactive to Manufactured” transition (see section 5.2.2.2.1) is Mandatory. This transition is accomplished via the Activate method (see section 5.1.1).

##### 5.2.2.2.1 Manufactured-Inactive to Manufactured

Triggers:

- The Activate method (see section 5.1.1) is successfully invoked on the SP’s object in the Admin SP’s SP table.

Side effects:

- The value in the LifeCycleState column of the SP’s object in the Admin SP’s SP table changes to Manufactured.
- The current SID PIN (C_PIN_SID) in the Admin SP is copied into the PIN column of Admin1’s C_PIN credential (C_PIN_Admin1) in the activated SP. This allows taking ownership of the SP with a known PIN credential.
- Any functionality enabled by the templates incorporated into the SP becomes active.

When the Locking SP transitions from the Manufactured-Inactive state to the Manufactured state (via invocation of the Activate method), the Storage Device SHALL NOT destroy any user data.

##### 5.2.2.2.2 ANY STATE to ORIGINAL FACTORY STATE

Triggers:

- Revert or RevertSP is successfully invoked on the SP.

Side effects:

- The value in the LifeCycleState column of the SP’s object in the Admin SP’s SP table changes to the value of the SP’s Original Factory State.
- The SP itself reverts to its Original Factory State, as described in sections 5.1.2 and 5.1.3.
- If the SP’s Original Factory State was Manufactured-Inactive, any functionality enabled by the templates incorporated into the SP becomes inactive.

#### 5.2.2.3 State behaviors for Manufactured SPs

##### 5.2.2.3.1 Manufactured-Inactive

Any functionality enabled by the templates incorporated into the SP is inactive in this state. Sessions cannot be opened to SPs in this state.

When the Locking SP is in the Manufactured-Inactive state, the Locking SP’s management of the Storage Device's locking and media encryption features SHALL be disabled.

##### 5.2.2.3.2 Manufactured

Behavior of an SP in the Manufactured state is identical to the behavior of an SP in the Issued state, as described in

[2].

When the Locking SP is in the Manufactured state, the Locking SP’s management of the Storage Device's locking and media encryption features SHALL be enabled.

### 5.2.3 Type Table Modification

In order to accommodate the additional life cycle states defined in this specification, the definition of the life_cycle_state type is changed from [2] to that described in Table 49:

**Table 49 - Life Cycle State Type Table Modification**

| UID | Name | Format | Size | Description |
| --- | --- | --- | --- | --- |
| 00 00 00 05  <br> 00 00 04 05 | life_cycle_state | Enumeration_Type,  <br> 0,  <br> 15 |  | Used to represent the current life cycle state.  The valid values are:    <br> = issued,   <br> = issued-disabled,   <br> = issued-frozen,   <br> = issued-disabled-frozen,   <br> = issued-failed,   <br> 5-7 = reserved,   <br> = manufactured-inactive,   <br> = manufactured,   <br> = manufactured-disabled,   <br> = manufactured-frozen,  12 = manufactured-disabledfrozen,   <br> 13 = manufactured-failed,  14-15 = reserved |

## 5.3 Byte Table Access Granularity

**Start of Informative Comment**

While the general architecture defined in [2] allows data to be written into byte tables starting at any arbitrary byte boundary and with any arbitrary byte length, certain types of Storage Devices work more efficiently when data is written aligned to a larger block boundary. This section defines extensions to [2] that allow a Storage Device to report the restrictions that it enforces when the host invokes the Set method on byte tables. **End of Informative Comment**

### 5.3.1 Table Table Modification

In order to allow a Storage Device to report its mandatory and recommended data alignment restrictions when accessing byte tables, the Table table SHALL contain the additional columns shown in Table 50.

The mandatory and recommended data alignment restrictions do not apply to Object tables. **Table 50 - Table Table Additional Columns**

| Column Number | Column Name | IsUnique | Column Type |
| --- | --- | --- | --- |
| 0x0D | MandatoryWriteGranularity |  | uinteger_4 |
| 0x0E | RecommendedAccessGranularity |  | uinteger_4 |

#### 5.3.1.1 MandatoryWriteGranularity

This column is used to report the granularity that the Storage Device enforces when the host invokes the Set method on byte tables.

This column SHALL NOT be modifiable by the host.

##### 5.3.1.1.1 Object Tables

For rows in the Table table that pertain to object tables, the value of the MandatoryWriteGranularity column SHALL be zero.

##### 5.3.1.1.2 Byte Tables

For rows in the Table table that pertain to byte tables, the MandatoryWriteGranularity column indicates the mandatory access granularity (in bytes) for the Set method for the table described in these rows of the Table table. The MandatoryWriteGranularity column indicates the alignment requirement for both the access start offset (the Where parameter) and length (number of bytes in the Values parameter).

The value of the MandatoryWriteGranularity column SHALL be less than or equal to the value in the RecommendedAccessGranularity column in the same row of the Table table.

The value of MandatoryWriteGranularity SHALL be less than or equal to 8192.

When the host invokes the Set method on a byte table, if ValidMandatoryGranularity (see Figure 4) is False, then the method SHALL fail with status INVALID_PARAMETER.

If the TPer does not have a requirement on mandatory alignment for the byte table described in a row of the Table table, then its MandatoryWriteGranularity column SHALL be set to one.

**Figure 4 - ValidMandatoryGranularity definition**

| For the Set method:  <br>    ValidMandatoryGranularity is True if  <br> (x modulo MandatoryWriteGranularity) = 0  <br>   <br>                   and  <br>   <br> (y modulo MandatoryWriteGranularity) = 0  <br>   <br> where:  <br>    x =  the start offset of the Set method   <br>             (i.e., the value of the Where parameter)    y = the number of data bytes being set   <br>             (i.e., the length of the Values parameter) |
| --- |

#### 5.3.1.2 RecommendedAccessGranularity

This column is used to report the granularity that the Storage Device recommends when the host invokes the Set or Get method on byte tables.

This column SHALL NOT be modifiable by the host.

##### 5.3.1.2.1 Object Tables

For rows in the Table table that pertain to object tables, the value of the RecommendedAccessGranularity column SHALL be zero.

##### 5.3.1.2.2 Byte Tables

For rows in the Table table that pertain to byte tables, the RecommendedAccessGranularity column indicates the recommended access granularity (in bytes) for the Set and Get method for the table described in these rows of the Table table. The RecommendedAccessGranularity column indicates the alignment of data for the Set and Get method that allows for optimal Set/Get performance.

If the TPer does not have a recommended alignment for the byte table described in a row of the Table table, then its RecommendedAccessGranularity column SHALL be set to one.

When the host invokes the Set method on a byte table, if ValidRecommendedGranularity (see Figure 5) is False, then the performance of the TPer MAY be reduced when processing the method.

**Figure 5 - ValidRecommendedGranularity definition for Set**

| For the Set method:  <br>    ValidRecommendedGranularity is True if  <br> (x modulo RecommendedAccessGranularity) = 0  <br>   <br>                   and  <br>   <br> (y modulo RecommendedAccessGranularity) = 0  <br>   <br> where:  <br>    x =  the start offset of the Set method   <br>             (i.e., the value of the Where parameter)    y = the number of data bytes being set   <br>             (i.e., the length of the Values parameter) |
| --- |

When the host invokes the Get method on a byte table, if ValidRecommendedGranularity (see Figure 6) is False, then the performance of the TPer MAY be reduced when processing the method.

**Figure 6 - ValidRecommendedGranularity definition for Get**

For the Get method:

ValidRecommendedGranularity is True if

1. (x modulo RecommendedAccessGranularity) = 0

and

2. (y modulo RecommendedAccessGranularity) = 0

where:

x = the start offset of the Get method

(i.e., the value of the startRow component of the Cellblock parameter) y = the number of data bytes being retrieved

(i.e., the difference of the endRow and startRow components of the Cellblock parameter, plus one)

## 5.4 Examples of Alignment Geometry Reporting

Figure 7 illustrates reporting for a typical legacy Storage Device where there is one logical block per physical block on the media.

**Figure 7 - Example: AlignmentGranularity=1, Lowest Aligned LBA=0**

| 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Alignment Granularity |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |

Figure 8 illustrates geometry for a Storage Device where there are 8 logical blocks per physical block (e.g., a 4K physical block) and the first logical block is aligned at the beginning of the first physical block.

**Figure 8 - Example: AlignmentGranularity=8, Lowest Aligned LBA=0**

| 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 | 18 | 19 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
|  | AlignmentGranularity |  |  |  |  |  |  |  | AlignmentGranularity |  |  |  |  |  |  |  | . . . |  |  |

Figure 9 illustrates geometry for a Storage Device where there are 8 logical blocks per physical block (e.g., a 4K physical block) and LBA=1 is the first logical block that is aligned at the beginning of a physical block

**Figure 9 - Example: AlignmentGranularity=8, Lowest Aligned LBA=1**

|  | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AlignmentGranularity |  |  | AlignmentGranularity |  |  |  |  |  |  |  | . . . |  |  |

Figure 10 illustrates geometry for a Storage Device where there are 2000 logical blocks per physical block and LBA=1234 is the first logical block that is aligned at the beginning of a physical block.

**Figure 10 - Example: AlignmentGranularity=2000, Lowest Aligned LBA=1234**

|  | 0 | . . . | 1230 | 1231 | 1232 | 1233 | 1234 | . . . | 3233 | 3234 | . . . |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AlignmentGranularity |  |  |  |  |  |  | AlignmentGranularity |  |  | . . . |  |
