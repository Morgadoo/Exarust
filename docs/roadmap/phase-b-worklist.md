# Phase B work list

Generated from the function ledger by `context/tools/ledger/make_worklist.py` (roadmap task A8).
Each open function is a row in `context/re/ledger/ledger.tsv` with its proof; lanes start there.

| class | functions | bytes | Phase B task |
|---|---|---|---|
| uncalled | 324 | 98,070 | B1 wire (or prove unreachable) |
| partial | 901 | 681,290 | B4 complete the port |
| missing | 37 | 6,211 | B4 port |
| unaudited | 0 | 0 | Phase A |

## By section (open bytes)

| section | partial | uncalled | missing |
|---|---|---|---|
| physics | 130,060 | 14,960 | 238 |
| render | 91,571 | 10,884 | 2,710 |
| ui | 69,344 | 32,066 | 167 |
| characters | 71,071 | 11,244 | 1,452 |
| modes | 57,853 | 7,005 | 363 |
| powers | 62,833 | 1,307 | 181 |
| world | 50,252 | 8,635 | 324 |
| ai | 57,286 | 776 | 11 |
| items | 38,648 | 4,077 | 525 |
| data | 18,368 | 3,068 | 0 |
| runtime | 16,733 | 1,029 | 181 |
| core | 9,818 | 2,319 | 59 |
| script | 3,796 | 700 | 0 |
| audio | 2,097 | 0 | 0 |
| combat | 1,560 | 0 | 0 |

## By component, largest first

| component | section | partial (fn / bytes) | uncalled (fn / bytes) | missing (fn / bytes) | open bytes |
|---|---|---|---|---|---|
| TMotileDynamics | physics | 32 / 88,168 | 2 / 315 | 0 / 0 | 88,483 |
| SG_CharacterGen (unit routines) | characters | 38 / 28,359 | 23 / 5,324 | 0 / 0 | 33,683 |
| TSimSector | physics | 23 / 20,137 | 0 / 0 | 0 / 0 | 20,137 |
| TMotilePart | characters | 29 / 14,896 | 4 / 1,296 | 2 / 1,370 | 17,562 |
| XA_Arena (unit routines) | modes | 23 / 15,343 | 3 / 1,321 | 4 / 363 | 17,027 |
| TVXGIManager | render | 11 / 16,981 | 0 / 0 | 0 / 0 | 16,981 |
| SG_PlayerControls (unit routines) | ai | 26 / 15,240 | 0 / 0 | 0 / 0 | 15,240 |
| TPathFinder | ai | 4 / 14,517 | 0 / 0 | 0 / 0 | 14,517 |
| TGUIDrawSkin | ui | 2 / 3,550 | 19 / 9,231 | 0 / 0 | 12,781 |
| FLX_Terrain_Texrender2 (unit routines) | render | 9 / 12,302 | 0 / 0 | 0 / 0 | 12,302 |
| SG_Character_Create (unit routines) | modes | 19 / 6,864 | 21 / 4,436 | 0 / 0 | 11,300 |
| RF_Skinning (unit routines) | characters | 3 / 10,728 | 1 / 189 | 0 / 0 | 10,917 |
| TGUIForm | ui | 19 / 8,872 | 10 / 1,715 | 0 / 0 | 10,587 |
| RF_Dynamics_Volumes (unit routines) | physics | 1 / 6,604 | 1 / 2,608 | 0 / 0 | 9,212 |
| TTendrilEmitter | powers | 2 / 9,208 | 0 / 0 | 0 / 0 | 9,208 |
| XA_SaveGames (unit routines) | modes | 14 / 8,988 | 1 / 128 | 0 / 0 | 9,116 |
| TMotileAI | ai | 14 / 7,782 | 1 / 142 | 1 / 11 | 7,935 |
| RF_3DView (unit routines) | render | 11 / 6,665 | 0 / 0 | 6 / 1,097 | 7,762 |
| FLX_GPUPrograms (unit routines) | render | 4 / 7,413 | 2 / 266 | 0 / 0 | 7,679 |
| TRBDDoor | world | 3 / 7,666 | 0 / 0 | 0 / 0 | 7,666 |
| TVCCloth | physics | 9 / 6,840 | 5 / 688 | 0 / 0 | 7,528 |
| TStaticTerrain | world | 6 / 7,120 | 1 / 201 | 0 / 0 | 7,321 |
| Character equipment & inventory routines (TCharacterView by address) | items | 11 / 7,253 | 0 / 0 | 0 / 0 | 7,253 |
| RF_OS_Functions (unit routines) | runtime | 9 / 6,381 | 4 / 851 | 0 / 0 | 7,232 |
| RF_SceneNodes (unit routines) | world | 5 / 6,798 | 2 / 333 | 0 / 0 | 7,131 |
| TTetraRBD | physics | 9 / 6,166 | 2 / 661 | 0 / 0 | 6,827 |
| Steam Input action bindings | runtime | 11 / 6,523 | 2 / 166 | 2 / 107 | 6,796 |
| RFE_GridEnvs (unit routines) | world | 7 / 6,790 | 0 / 0 | 0 / 0 | 6,790 |
| TGUIItemView | ui | 1 / 4,979 | 9 / 1,562 | 0 / 0 | 6,541 |
| FLX_Content_Stream (unit routines) | data | 9 / 6,005 | 4 / 458 | 0 / 0 | 6,463 |
| TLoadoutForm | modes | 5 / 6,248 | 1 / 80 | 0 / 0 | 6,328 |
| RFE_Export_RFC2 (unit routines) | data | 7 / 5,868 | 2 / 240 | 0 / 0 | 6,108 |
| TMatchBoard | modes | 3 / 6,102 | 0 / 0 | 0 / 0 | 6,102 |
| RF_Navigation (unit routines) | render | 6 / 4,181 | 7 / 1,804 | 0 / 0 | 5,985 |
| RF_GUI_Main (unit routines) | ui | 8 / 4,651 | 6 / 1,187 | 1 / 23 | 5,861 |
| TCompactSkillUI | ui | 0 / 0 | 11 / 5,838 | 0 / 0 | 5,838 |
| RF_GUI_Controls (unit routines) | ui | 7 / 4,821 | 2 / 881 | 0 / 0 | 5,702 |
| TPlayerBehaviour | ai | 2 / 5,670 | 0 / 0 | 0 / 0 | 5,670 |
| TRenderer | render | 13 / 5,644 | 0 / 0 | 1 / 10 | 5,654 |
| TAISearchActivity | ai | 5 / 5,505 | 0 / 0 | 0 / 0 | 5,505 |
| SG_GameMain (unit routines) | modes | 14 / 5,443 | 0 / 0 | 0 / 0 | 5,443 |
| SG_GameGUI (unit routines) | ui | 8 / 5,341 | 0 / 0 | 0 / 0 | 5,341 |
| TIconManager | ui | 0 / 0 | 5 / 5,304 | 0 / 0 | 5,304 |
| TParticleSystem | powers | 2 / 4,676 | 1 / 210 | 0 / 0 | 4,886 |
| TCharacterView | ui | 5 / 4,016 | 1 / 866 | 0 / 0 | 4,882 |
| Rigid-body shape builders (RF_GUI_Controls_Ex (unit routines) by address) | physics | 0 / 0 | 2 / 4,570 | 1 / 238 | 4,808 |
| TTendrilEffect | powers | 1 / 4,804 | 0 / 0 | 0 / 0 | 4,804 |
| RF_Morphing (unit routines) | characters | 2 / 4,764 | 0 / 0 | 0 / 0 | 4,764 |
| RF_Dynamics (unit routines) | physics | 4 / 1,692 | 7 / 2,973 | 0 / 0 | 4,665 |
| TNavCameraControl | render | 1 / 4,401 | 0 / 0 | 0 / 0 | 4,401 |
| RF_HW_Config (unit routines) | ui | 6 / 4,195 | 3 / 198 | 0 / 0 | 4,393 |
| TSkillUI | ui | 4 / 3,673 | 2 / 716 | 0 / 0 | 4,389 |
| FLX_Character_Textures (unit routines) | characters | 2 / 4,094 | 1 / 187 | 0 / 0 | 4,281 |
| TTerrainManager | world | 6 / 3,351 | 2 / 923 | 0 / 0 | 4,274 |
| TGSLines | ui | 2 / 3,540 | 2 / 714 | 0 / 0 | 4,254 |
| FLX_System (unit routines) | core | 20 / 1,840 | 14 / 2,319 | 1 / 59 | 4,218 |
| SG_Thaum_DIsp (unit routines) | powers | 10 / 3,765 | 1 / 431 | 0 / 0 | 4,196 |
| SG_CharacterAI (unit routines) | ai | 11 / 4,122 | 0 / 0 | 0 / 0 | 4,122 |
| T3DViewport | ui | 6 / 2,963 | 2 / 1,101 | 0 / 0 | 4,064 |
| TTerrainGrass | world | 3 / 4,057 | 0 / 0 | 0 / 0 | 4,057 |
| TSkinController2 | characters | 2 / 3,024 | 1 / 983 | 0 / 0 | 4,007 |
| SG_Character_Data (unit routines) | characters | 4 / 1,601 | 8 / 2,307 | 0 / 0 | 3,908 |
| RF_Materials (unit routines) | render | 2 / 3,866 | 0 / 0 | 0 / 0 | 3,866 |
| RFT_BoundSphere (unit routines) | core | 2 / 3,851 | 0 / 0 | 0 / 0 | 3,851 |
| TMesh | render | 4 / 3,114 | 2 / 703 | 0 / 0 | 3,817 |
| TProcMaterial | items | 4 / 3,148 | 0 / 0 | 1 / 525 | 3,673 |
| SG_ItemGUI (unit routines) | items | 3 / 2,553 | 4 / 1,049 | 0 / 0 | 3,602 |
| FLX_Materials_Procedural (unit routines) | items | 2 / 3,578 | 0 / 0 | 0 / 0 | 3,578 |
| TCombatant | ai | 6 / 3,234 | 2 / 268 | 0 / 0 | 3,502 |
| TPwrForceKinesis_S | powers | 5 / 3,452 | 0 / 0 | 0 / 0 | 3,452 |
| RF_Intersection (unit routines) | physics | 2 / 288 | 8 / 3,137 | 0 / 0 | 3,425 |
| RF_Textures (unit routines) | render | 9 / 2,661 | 4 / 670 | 0 / 0 | 3,331 |
| TGUIItemContainer | items | 1 / 1,847 | 1 / 1,476 | 0 / 0 | 3,323 |
| RF_Mesh (unit routines) | render | 1 / 1,108 | 1 / 2,210 | 0 / 0 | 3,318 |
| Engine math (vectors, matrices, fast trig) | core | 12 / 3,295 | 0 / 0 | 0 / 0 | 3,295 |
| RF_GUI_Controls_Ex (unit routines) | ui | 7 / 3,277 | 0 / 0 | 0 / 0 | 3,277 |
| TShadowMapDirect | render | 4 / 3,259 | 0 / 0 | 0 / 0 | 3,259 |
| TFireEffectRow | powers | 3 / 3,114 | 0 / 0 | 0 / 0 | 3,114 |
| TGeometry | render | 6 / 2,254 | 0 / 0 | 1 / 804 | 3,058 |
| RF_Terrain_Materials (unit routines) | world | 1 / 2,474 | 3 / 484 | 0 / 0 | 2,958 |
| TCharCreation | modes | 6 / 2,108 | 5 / 818 | 0 / 0 | 2,926 |
| RF_DBObjects (unit routines) | data | 2 / 2,672 | 2 / 202 | 0 / 0 | 2,874 |
| TLightSource | render | 5 / 2,369 | 1 / 494 | 0 / 0 | 2,863 |
| RF_Main (unit routines) | runtime | 6 / 2,711 | 0 / 0 | 0 / 0 | 2,711 |
| SG_Thaum_Force (unit routines) | powers | 4 / 2,521 | 0 / 0 | 1 / 181 | 2,702 |
| TItemGUIManager | items | 6 / 1,690 | 4 / 903 | 0 / 0 | 2,593 |
| TGUITableView | ui | 1 / 2,523 | 0 / 0 | 0 / 0 | 2,523 |
| FLX_IconRender (unit routines) | render | 0 / 0 | 7 / 2,195 | 2 / 286 | 2,481 |
| RF_kdTrees (unit routines) | world | 10 / 2,369 | 0 / 0 | 0 / 0 | 2,369 |
| TForceShellEffect | powers | 2 / 2,243 | 0 / 0 | 0 / 0 | 2,243 |
| TAudioEnvironment | audio | 3 / 2,097 | 0 / 0 | 0 / 0 | 2,097 |
| TTextRenderer | ui | 1 / 2,043 | 0 / 0 | 0 / 0 | 2,043 |
| TInventory | items | 5 / 1,909 | 1 / 41 | 0 / 0 | 1,950 |
| TClassSelect | modes | 5 / 1,940 | 0 / 0 | 0 / 0 | 1,940 |
| TForceBarricadeEffect | powers | 1 / 1,922 | 0 / 0 | 0 / 0 | 1,922 |
| TkdTree | world | 3 / 1,661 | 1 / 240 | 0 / 0 | 1,901 |
| TForceBarrierEffect | powers | 1 / 1,894 | 0 / 0 | 0 / 0 | 1,894 |
| SG_Dynamics_Effects (unit routines) | powers | 3 / 1,876 | 0 / 0 | 0 / 0 | 1,876 |
| SG_ObjectDB2 (unit routines) | data | 4 / 1,738 | 1 / 125 | 0 / 0 | 1,863 |
| TDivCardForm | modes | 1 / 1,863 | 0 / 0 | 0 / 0 | 1,863 |
| TArsenalView | items | 2 / 1,249 | 2 / 608 | 0 / 0 | 1,857 |
| TGridEnvManager | world | 2 / 1,844 | 0 / 0 | 0 / 0 | 1,844 |
| Shading-mask GLSL snippets | render | 1 / 1,830 | 0 / 0 | 0 / 0 | 1,830 |
| RFE_MeshTools (unit routines) | render | 1 / 37 | 4 / 1,772 | 0 / 0 | 1,809 |
| FLX_ItemFactory (unit routines) | items | 2 / 1,794 | 0 / 0 | 0 / 0 | 1,794 |
| Scene content helpers (unit before RF_Dynamics) | world | 1 / 925 | 4 / 768 | 0 / 0 | 1,693 |
| RF_Env_Effects (unit routines) | powers | 1 / 1,666 | 0 / 0 | 0 / 0 | 1,666 |
| Item variant & condition randomisers (FLX_Character_Textures (unit routines) by address) | items | 1 / 1,662 | 0 / 0 | 0 / 0 | 1,662 |
| TSceneEnvironment | world | 2 / 1,658 | 0 / 0 | 0 / 0 | 1,658 |
| TPwrDispLongDoor_I | powers | 2 / 1,649 | 0 / 0 | 0 / 0 | 1,649 |
| TTransitionScreen | ui | 3 / 1,605 | 0 / 0 | 0 / 0 | 1,605 |
| TSelectionUI | ui | 3 / 1,545 | 0 / 0 | 0 / 0 | 1,545 |
| TTErrainkdTree | world | 0 / 0 | 5 / 1,513 | 0 / 0 | 1,513 |
| SG_Thaum_Mind (unit routines) | powers | 3 / 1,121 | 1 / 375 | 0 / 0 | 1,496 |
| TWeaponBlood | combat | 2 / 1,494 | 0 / 0 | 0 / 0 | 1,494 |
| TPwrForceThrow_I | powers | 3 / 1,309 | 1 / 170 | 0 / 0 | 1,479 |
| TRTDialogue | script | 1 / 1,461 | 0 / 0 | 0 / 0 | 1,461 |
| Screen-quad GLSL snippets | render | 5 / 1,457 | 0 / 0 | 0 / 0 | 1,457 |
| TShadowCubeMap | render | 4 / 1,439 | 0 / 0 | 0 / 0 | 1,439 |
| RF_MeshVoxels (unit routines) | render | 1 / 1,434 | 0 / 0 | 0 / 0 | 1,434 |
| RF_Dynamics_Animation (unit routines) | characters | 3 / 1,343 | 0 / 0 | 1 / 82 | 1,425 |
| TTerrainMesh | render | 3 / 1,408 | 0 / 0 | 0 / 0 | 1,408 |
| RF_Polyhedra (unit routines) | render | 1 / 1,395 | 0 / 0 | 0 / 0 | 1,395 |
| Equipment effects and focus-item slot (RFE_Effects_Electricity (unit routines) by address) | characters | 5 / 1,151 | 1 / 167 | 0 / 0 | 1,318 |
| RFE_MeshSubdivide (unit routines) | render | 4 / 1,186 | 0 / 0 | 1 / 96 | 1,282 |
| TScene | world | 1 / 740 | 4 / 535 | 0 / 0 | 1,275 |
| RF_SceneEditor (unit routines) | world | 1 / 234 | 1 / 1,031 | 0 / 0 | 1,265 |
| TPwrForceStrikes_C | powers | 2 / 1,259 | 0 / 0 | 0 / 0 | 1,259 |
| SG_Thaumaturgy (unit routines) | powers | 3 / 1,227 | 0 / 0 | 0 / 0 | 1,227 |
| TGUIDrawSkinTex | ui | 3 / 1,209 | 0 / 0 | 0 / 0 | 1,209 |
| TPwrDispVanish_I | powers | 2 / 1,175 | 0 / 0 | 0 / 0 | 1,175 |
| THairColPicker | ui | 2 / 1,159 | 0 / 0 | 0 / 0 | 1,159 |
| TBlastEntity | powers | 2 / 1,124 | 0 / 0 | 0 / 0 | 1,124 |
| Golem guardians and alarm zones (RFE_Effects_Electricity (unit routines) by address) | characters | 3 / 1,111 | 0 / 0 | 0 / 0 | 1,111 |
| TGridEnvBuilder | world | 1 / 94 | 4 / 1,004 | 0 / 0 | 1,098 |
| SG_TradeSystem (unit routines) | modes | 4 / 1,029 | 1 / 69 | 0 / 0 | 1,098 |
| XA_ExperienceSys (unit routines) | modes | 1 / 936 | 2 / 153 | 0 / 0 | 1,089 |
| TShellEffect | powers | 2 / 1,068 | 0 / 0 | 0 / 0 | 1,068 |
| RFE_Effects_Electricity (unit routines) | powers | 2 / 1,050 | 0 / 0 | 0 / 0 | 1,050 |
| SG_Teleporters (unit routines) | world | 2 / 1,042 | 0 / 0 | 0 / 0 | 1,042 |
| Texture compression formats (DXT/LATC) | render | 4 / 1,031 | 0 / 0 | 0 / 0 | 1,031 |
| TBubbleEffect | powers | 1 / 1,031 | 0 / 0 | 0 / 0 | 1,031 |
| TForceBlastEffect | powers | 1 / 948 | 0 / 0 | 0 / 0 | 948 |
| THairEditForm | ui | 1 / 828 | 1 / 117 | 0 / 0 | 945 |
| RF_DataIO (unit routines) | data | 0 / 0 | 3 / 936 | 0 / 0 | 936 |
| TPostProcessor | render | 3 / 929 | 0 / 0 | 0 / 0 | 929 |
| TSceneTaskInt | world | 0 / 0 | 2 / 903 | 0 / 0 | 903 |
| RF_ShadowMap (unit routines) | render | 2 / 771 | 1 / 127 | 0 / 0 | 898 |
| TForceBlastGeometry | powers | 2 / 890 | 0 / 0 | 0 / 0 | 890 |
| FG_AIFunctions (unit routines) | script | 6 / 888 | 0 / 0 | 0 / 0 | 888 |
| RF_Dynamics_Game (unit routines) | world | 2 / 575 | 2 / 312 | 0 / 0 | 887 |
| FLX_EffectRender (unit routines) | powers | 1 / 887 | 0 / 0 | 0 / 0 | 887 |
| RF_Import_RFC2 (unit routines) | data | 3 / 490 | 1 / 386 | 0 / 0 | 876 |
| TForceTorrentEffect | powers | 1 / 811 | 0 / 0 | 0 / 0 | 811 |
| TPwrSpiritForm | powers | 2 / 803 | 0 / 0 | 0 / 0 | 803 |
| TMainMenu | ui | 2 / 799 | 0 / 0 | 0 / 0 | 799 |
| TBarricadeEntity | powers | 1 / 787 | 0 / 0 | 0 / 0 | 787 |
| Character roles & relations (script functions) | ai | 2 / 464 | 1 / 318 | 0 / 0 | 782 |
| TRendererShaderFactory | render | 1 / 684 | 1 / 85 | 0 / 0 | 769 |
| SG_GameFunctions (unit routines) | script | 3 / 445 | 1 / 317 | 0 / 0 | 762 |
| TGemstone | items | 1 / 760 | 0 / 0 | 0 / 0 | 760 |
| TResin | items | 1 / 752 | 0 / 0 | 0 / 0 | 752 |
| TPwrDispSummonDemon_I | powers | 1 / 752 | 0 / 0 | 0 / 0 | 752 |
| TSteelPainted | items | 1 / 748 | 0 / 0 | 0 / 0 | 748 |
| TSteelChain | items | 1 / 732 | 0 / 0 | 0 / 0 | 732 |
| TScriptRole | script | 1 / 712 | 0 / 0 | 0 / 0 | 712 |
| TSteelGen | items | 1 / 700 | 0 / 0 | 0 / 0 | 700 |
| TIronGen | items | 1 / 700 | 0 / 0 | 0 / 0 | 700 |
| TSteelBlade | items | 1 / 692 | 0 / 0 | 0 / 0 | 692 |
| TSteelPlate2 | items | 1 / 692 | 0 / 0 | 0 / 0 | 692 |
| FPC System RTL | runtime | 6 / 675 | 0 / 0 | 0 / 0 | 675 |
| RF_ObjectLibs (unit routines) | data | 2 / 662 | 0 / 0 | 0 / 0 | 662 |
| TPickerForm | ui | 1 / 184 | 2 / 473 | 0 / 0 | 657 |
| TForceTorrentGeometry | powers | 1 / 651 | 0 / 0 | 0 / 0 | 651 |
| TTendrilGeometry | powers | 1 / 651 | 0 / 0 | 0 / 0 | 651 |
| TPwrControlMind_C | powers | 1 / 646 | 0 / 0 | 0 / 0 | 646 |
| TStrikesGeometry | powers | 1 / 646 | 0 / 0 | 0 / 0 | 646 |
| XA_HelpSystem (unit routines) | ui | 0 / 0 | 1 / 619 | 0 / 0 | 619 |
| TStringLibrary | data | 0 / 0 | 4 / 615 | 0 / 0 | 615 |
| FLX_Character_Motions (unit routines) | characters | 0 / 0 | 3 / 613 | 0 / 0 | 613 |
| RF_Terrain (unit routines) | world | 1 / 609 | 0 / 0 | 0 / 0 | 609 |
| TForceWaveEntity | powers | 1 / 603 | 0 / 0 | 0 / 0 | 603 |
| GUI support routines (unit before RF_GUI_Main) | ui | 0 / 0 | 1 / 564 | 0 / 0 | 564 |
| Colour helpers and parallel-for dispatch (RF_kdTrees (unit routines) by address) | core | 2 / 563 | 0 / 0 | 0 / 0 | 563 |
| TToolTip | ui | 1 / 553 | 0 / 0 | 0 / 0 | 553 |
| TPackageLibrary | data | 1 / 542 | 0 / 0 | 0 / 0 | 542 |
| TLightAlias | render | 2 / 537 | 0 / 0 | 0 / 0 | 537 |
| TClothSilk | items | 1 / 536 | 0 / 0 | 0 / 0 | 536 |
| TClothPadded1 | items | 1 / 536 | 0 / 0 | 0 / 0 | 536 |
| TClothPadded2 | items | 1 / 536 | 0 / 0 | 0 / 0 | 536 |
| TClothVelvet | items | 1 / 528 | 0 / 0 | 0 / 0 | 528 |
| Program main (Exanima.lpr) | runtime | 3 / 443 | 0 / 0 | 1 / 74 | 517 |
| TClothBasic | items | 1 / 516 | 0 / 0 | 0 / 0 | 516 |
| TClothFine | items | 1 / 508 | 0 / 0 | 0 / 0 | 508 |
| TGUITask | ui | 3 / 504 | 0 / 0 | 0 / 0 | 504 |
| TEndSessionMenu | modes | 1 / 477 | 0 / 0 | 0 / 0 | 477 |
| TCharViewBehaviour | ai | 1 / 466 | 0 / 0 | 0 / 0 | 466 |
| Item description phrases | items | 1 / 465 | 0 / 0 | 0 / 0 | 465 |
| TLeatherA01 | items | 1 / 464 | 0 / 0 | 0 / 0 | 464 |
| TLeatherA02 | items | 1 / 464 | 0 / 0 | 0 / 0 | 464 |
| TPhysiquePicker | ui | 4 / 464 | 0 / 0 | 0 / 0 | 464 |
| TSuedeA01 | items | 1 / 456 | 0 / 0 | 0 / 0 | 456 |
| TWiredGripA01 | items | 1 / 456 | 0 / 0 | 0 / 0 | 456 |
| TTexture | render | 1 / 453 | 0 / 0 | 0 / 0 | 453 |
| THelpScreen | ui | 2 / 177 | 2 / 199 | 2 / 77 | 453 |
| SGD_GameObjEdit (unit routines) | ui | 1 / 440 | 0 / 0 | 0 / 0 | 440 |
| SG_Prelude_Main (unit routines) | modes | 1 / 422 | 0 / 0 | 0 / 0 | 422 |
| TThaumTree | powers | 3 / 276 | 1 / 121 | 0 / 0 | 397 |
| Scene node despawn and viewport step (TSceneTaskInt by address) | world | 0 / 0 | 1 / 388 | 0 / 0 | 388 |
| RFG_Dialogue (unit routines) | script | 0 / 0 | 2 / 383 | 0 / 0 | 383 |
| TProjectManager | data | 1 / 373 | 0 / 0 | 0 / 0 | 373 |
| FLX_PostProcess (unit routines) | render | 1 / 222 | 0 / 0 | 1 / 127 | 349 |
| TQuadFBO | render | 2 / 328 | 0 / 0 | 0 / 0 | 328 |
| T3DNode | world | 0 / 0 | 0 / 0 | 1 / 324 | 324 |
| TCharLoot | items | 1 / 311 | 0 / 0 | 0 / 0 | 311 |
| Fire particle presets | powers | 2 / 309 | 0 / 0 | 0 / 0 | 309 |
| TReadKeyScreen | ui | 1 / 303 | 0 / 0 | 0 / 0 | 303 |
| RF_GLGeometry (unit routines) | render | 0 / 0 | 0 / 0 | 1 / 290 | 290 |
| FLX_GUI_Graphics (unit routines) | ui | 2 / 289 | 0 / 0 | 0 / 0 | 289 |
| RF_GUI_Text_Edit (unit routines) | ui | 0 / 0 | 1 / 270 | 0 / 0 | 270 |
| Core string/number formatting utilities | core | 1 / 269 | 0 / 0 | 0 / 0 | 269 |
| FLXG_Pathfinding (unit routines) | ai | 2 / 265 | 0 / 0 | 0 / 0 | 265 |
| SG_GUI_Skills (unit routines) | ui | 0 / 0 | 1 / 197 | 1 / 67 | 264 |
| TSector | world | 1 / 245 | 0 / 0 | 0 / 0 | 245 |
| TPopUpContainer | items | 1 / 245 | 0 / 0 | 0 / 0 | 245 |
| TCacheShaderScreen | ui | 1 / 242 | 0 / 0 | 0 / 0 | 242 |
| TGameTips | script | 1 / 229 | 0 / 0 | 0 / 0 | 229 |
| TCamera | render | 1 / 212 | 0 / 0 | 0 / 0 | 212 |
| TDropDownList | ui | 2 / 210 | 0 / 0 | 0 / 0 | 210 |
| RF_Material_Manager (unit routines) | render | 0 / 0 | 1 / 180 | 0 / 0 | 180 |
| TMotileBehaviour | characters | 0 / 0 | 3 / 178 | 0 / 0 | 178 |
| TAbstractToolTip | ui | 0 / 0 | 2 / 175 | 0 / 0 | 175 |
| TPopUpDesc | items | 2 / 168 | 0 / 0 | 0 / 0 | 168 |
| RFE_Dynamics_Cloth (unit routines) | physics | 1 / 165 | 0 / 0 | 0 / 0 | 165 |
| TPopupParchment | ui | 1 / 163 | 0 / 0 | 0 / 0 | 163 |
| RF_Renderer (unit routines) | render | 0 / 0 | 2 / 160 | 0 / 0 | 160 |
| TTextBox | ui | 0 / 0 | 1 / 139 | 0 / 0 | 139 |
| RFP_Mipmaps_Kaiser (unit routines) | render | 0 / 0 | 2 / 124 | 0 / 0 | 124 |
| SG_Character_DBs (unit routines) | data | 1 / 18 | 1 / 106 | 0 / 0 | 124 |
| TCharBaseForm | ui | 1 / 102 | 0 / 0 | 0 / 0 | 102 |
| .rfi image codec | render | 0 / 0 | 1 / 94 | 0 / 0 | 94 |
| SG_EmblemGen (unit routines) | modes | 1 / 90 | 0 / 0 | 0 / 0 | 90 |
| TCharEditForm | ui | 1 / 84 | 0 / 0 | 0 / 0 | 84 |
| TEntity | ai | 1 / 21 | 1 / 48 | 0 / 0 | 69 |
| RFE_Effects_Blood (unit routines) | combat | 1 / 66 | 0 / 0 | 0 / 0 | 66 |
| TScriptActor | script | 1 / 61 | 0 / 0 | 0 / 0 | 61 |
| RF_GUI_Taskbar (unit routines) | ui | 1 / 26 | 0 / 0 | 0 / 0 | 26 |
| TScanEffect | powers | 1 / 19 | 0 / 0 | 0 / 0 | 19 |
| RF_GUI_3DViewport (unit routines) | ui | 1 / 14 | 0 / 0 | 0 / 0 | 14 |
| Steam API glue | runtime | 0 / 0 | 1 / 12 | 0 / 0 | 12 |
| TPhysicsController | physics | 0 / 0 | 1 / 8 | 0 / 0 | 8 |
