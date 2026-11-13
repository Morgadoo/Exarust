# Phase B work list

Generated from the function ledger by `context/tools/ledger/make_worklist.py` (roadmap task A8).
Each open function is a row in `context/re/ledger/ledger.tsv` with its proof; lanes start there.

| class | functions | bytes | Phase B task |
|---|---|---|---|
| uncalled | 1,014 | 286,586 | B1 wire (or prove unreachable) |
| partial | 1,579 | 993,295 | B4 complete the port |
| missing | 72 | 15,780 | B4 port |
| unaudited | 0 | 0 | Phase A |

## By section (open bytes)

| section | partial | uncalled | missing |
|---|---|---|---|
| physics | 159,383 | 27,483 | 2,065 |
| ui | 97,130 | 86,693 | 1,695 |
| render | 128,158 | 43,078 | 3,232 |
| powers | 113,666 | 13,005 | 3,049 |
| ai | 110,222 | 9,587 | 407 |
| characters | 90,418 | 23,826 | 2,097 |
| modes | 82,006 | 20,201 | 942 |
| world | 66,182 | 21,393 | 563 |
| items | 72,908 | 13,172 | 670 |
| data | 32,023 | 13,873 | 0 |
| runtime | 16,565 | 7,524 | 990 |
| core | 13,668 | 4,678 | 59 |
| script | 5,740 | 1,803 | 11 |
| audio | 3,025 | 270 | 0 |
| combat | 2,201 | 0 | 0 |

## By component, largest first

| component | section | partial (fn / bytes) | uncalled (fn / bytes) | missing (fn / bytes) | open bytes |
|---|---|---|---|---|---|
| TMotileDynamics | physics | 48 / 99,121 | 5 / 5,469 | 1 / 38 | 104,628 |
| TMotileAI | ai | 31 / 35,910 | 7 / 3,725 | 3 / 360 | 39,995 |
| SG_CharacterGen (unit routines) | characters | 47 / 30,266 | 25 / 5,384 | 0 / 0 | 35,650 |
| XA_Arena (unit routines) | modes | 47 / 25,993 | 9 / 1,729 | 8 / 942 | 28,664 |
| SG_PlayerControls (unit routines) | ai | 44 / 23,983 | 6 / 993 | 1 / 47 | 25,023 |
| TSimSector | physics | 33 / 23,628 | 1 / 224 | 0 / 0 | 23,852 |
| TVCCloth | physics | 20 / 19,103 | 8 / 2,551 | 1 / 1,638 | 23,292 |
| TGSLines | ui | 4 / 14,830 | 13 / 6,492 | 0 / 0 | 21,322 |
| TGUIDrawSkin | ui | 4 / 3,994 | 39 / 16,713 | 0 / 0 | 20,707 |
| TMotilePart | characters | 36 / 17,198 | 10 / 1,909 | 2 / 1,370 | 20,477 |
| TPathFinder | ai | 7 / 16,983 | 4 / 2,334 | 0 / 0 | 19,317 |
| RF_SceneNodes (unit routines) | world | 12 / 15,946 | 13 / 2,520 | 0 / 0 | 18,466 |
| TGUIForm | ui | 30 / 9,518 | 32 / 8,274 | 1 / 153 | 17,945 |
| TGUIItemContainer | items | 16 / 13,877 | 6 / 3,483 | 0 / 0 | 17,360 |
| TVXGIManager | render | 12 / 16,995 | 1 / 61 | 0 / 0 | 17,056 |
| TMatchBoard | modes | 11 / 9,965 | 8 / 5,999 | 0 / 0 | 15,964 |
| FLX_GPUPrograms (unit routines) | render | 10 / 8,729 | 28 / 6,589 | 0 / 0 | 15,318 |
| TMesh | render | 20 / 13,220 | 4 / 1,746 | 0 / 0 | 14,966 |
| RF_Morphing (unit routines) | characters | 7 / 7,937 | 9 / 6,346 | 1 / 154 | 14,437 |
| XA_SaveGames (unit routines) | modes | 25 / 12,816 | 8 / 1,162 | 0 / 0 | 13,978 |
| TGUIItemView | ui | 0 / 0 | 27 / 12,862 | 0 / 0 | 12,862 |
| RF_Skinning (unit routines) | characters | 8 / 12,301 | 3 / 515 | 0 / 0 | 12,816 |
| TShaderBuilder | render | 0 / 0 | 13 / 12,787 | 0 / 0 | 12,787 |
| SG_GameGUI (unit routines) | ui | 20 / 10,427 | 12 / 2,147 | 2 / 183 | 12,757 |
| FLX_Terrain_Texrender2 (unit routines) | render | 9 / 12,302 | 0 / 0 | 0 / 0 | 12,302 |
| TForceTorrentEffect | powers | 6 / 11,171 | 0 / 0 | 1 / 728 | 11,899 |
| SG_Character_Create (unit routines) | modes | 19 / 6,864 | 21 / 4,436 | 0 / 0 | 11,300 |
| SG_Thaum_Mind (unit routines) | powers | 5 / 2,267 | 22 / 8,942 | 0 / 0 | 11,209 |
| TStaticTerrain | world | 12 / 10,617 | 3 / 522 | 0 / 0 | 11,139 |
| Character equipment & inventory routines (TCharacterView by address) | items | 17 / 8,995 | 9 / 1,514 | 0 / 0 | 10,509 |
| Steam Input action bindings | runtime | 8 / 4,285 | 14 / 5,335 | 5 / 867 | 10,487 |
| TForceBarricadeEffect | powers | 5 / 10,372 | 0 / 0 | 0 / 0 | 10,372 |
| SG_ItemGUI (unit routines) | items | 20 / 7,450 | 10 / 2,699 | 0 / 0 | 10,149 |
| TTendrilEmitter | powers | 6 / 9,672 | 1 / 67 | 0 / 0 | 9,739 |
| TAISearchActivity | ai | 14 / 9,316 | 3 / 212 | 0 / 0 | 9,528 |
| TTextBox | ui | 1 / 230 | 17 / 8,985 | 0 / 0 | 9,215 |
| RF_Dynamics_Volumes (unit routines) | physics | 1 / 6,604 | 1 / 2,608 | 0 / 0 | 9,212 |
| TTetraRBD | physics | 17 / 8,115 | 4 / 917 | 1 / 151 | 9,183 |
| FLX_Content_Stream (unit routines) | data | 16 / 7,762 | 13 / 1,300 | 0 / 0 | 9,062 |
| SG_GameMain (unit routines) | modes | 21 / 7,606 | 3 / 1,300 | 0 / 0 | 8,906 |
| RF_GUI_Main (unit routines) | ui | 7 / 5,075 | 19 / 2,486 | 3 / 1,167 | 8,728 |
| RF_Dynamics (unit routines) | physics | 8 / 2,263 | 22 / 6,364 | 0 / 0 | 8,627 |
| RF_OS_Functions (unit routines) | runtime | 13 / 6,781 | 5 / 1,749 | 1 / 49 | 8,579 |
| RF_3DView (unit routines) | render | 14 / 7,385 | 0 / 0 | 7 / 1,146 | 8,531 |
| FLX_Materials_Procedural (unit routines) | items | 9 / 5,600 | 11 / 2,821 | 1 / 76 | 8,497 |
| RFE_Export_RFC2 (unit routines) | data | 13 / 8,152 | 2 / 240 | 0 / 0 | 8,392 |
| TTendrilEffect | powers | 2 / 8,075 | 0 / 0 | 0 / 0 | 8,075 |
| SG_ObjectDB2 (unit routines) | data | 8 / 4,937 | 25 / 3,098 | 0 / 0 | 8,035 |
| TPlayerBehaviour | ai | 9 / 7,954 | 2 / 79 | 0 / 0 | 8,033 |
| TRBDDoor | world | 4 / 5,922 | 1 / 2,085 | 0 / 0 | 8,007 |
| SG_Character_Data (unit routines) | characters | 6 / 2,244 | 40 / 5,718 | 0 / 0 | 7,962 |
| TCharacterView | ui | 11 / 5,987 | 6 / 1,433 | 1 / 48 | 7,468 |
| FLX_System (unit routines) | core | 34 / 2,808 | 31 / 4,356 | 1 / 59 | 7,223 |
| FLX_Character_Textures (unit routines) | characters | 14 / 6,969 | 1 / 187 | 0 / 0 | 7,156 |
| TGUITableView | ui | 4 / 3,946 | 17 / 3,168 | 0 / 0 | 7,114 |
| RF_GUI_Controls (unit routines) | ui | 10 / 5,854 | 4 / 1,215 | 0 / 0 | 7,069 |
| RF_Navigation (unit routines) | render | 7 / 4,895 | 9 / 2,080 | 0 / 0 | 6,975 |
| TSkillUI | ui | 4 / 3,673 | 6 / 3,245 | 0 / 0 | 6,918 |
| TRenderer | render | 15 / 6,335 | 1 / 526 | 1 / 10 | 6,871 |
| RFE_GridEnvs (unit routines) | world | 7 / 6,790 | 0 / 0 | 0 / 0 | 6,790 |
| Character roles & relations (script functions) | ai | 11 / 6,145 | 2 / 603 | 0 / 0 | 6,748 |
| TParticleSystem | powers | 4 / 4,883 | 7 / 1,858 | 0 / 0 | 6,741 |
| RF_Textures (unit routines) | render | 17 / 5,574 | 9 / 1,051 | 0 / 0 | 6,625 |
| TLoadoutForm | modes | 7 / 6,452 | 1 / 80 | 0 / 0 | 6,532 |
| Engine math (vectors, matrices, fast trig) | core | 23 / 5,938 | 1 / 235 | 0 / 0 | 6,173 |
| T3DViewport | ui | 19 / 4,915 | 2 / 1,101 | 0 / 0 | 6,016 |
| TNavCameraControl | render | 2 / 5,215 | 3 / 772 | 0 / 0 | 5,987 |
| TForceBlastEffect | powers | 3 / 5,947 | 0 / 0 | 0 / 0 | 5,947 |
| TCompactSkillUI | ui | 0 / 0 | 11 / 5,838 | 0 / 0 | 5,838 |
| TCombatant | ai | 9 / 4,839 | 5 / 952 | 0 / 0 | 5,791 |
| TProcMaterial | items | 9 / 5,114 | 0 / 0 | 1 / 525 | 5,639 |
| TIconManager | ui | 0 / 0 | 6 / 5,573 | 0 / 0 | 5,573 |
| TSkinController2 | characters | 6 / 3,997 | 2 / 1,193 | 0 / 0 | 5,190 |
| Item description phrases | items | 5 / 4,855 | 2 / 288 | 0 / 0 | 5,143 |
| RF_Mesh (unit routines) | render | 3 / 2,922 | 1 / 2,210 | 0 / 0 | 5,132 |
| RF_HW_Config (unit routines) | ui | 8 / 4,879 | 3 / 198 | 0 / 0 | 5,077 |
| RF_Intersection (unit routines) | physics | 2 / 288 | 12 / 4,765 | 0 / 0 | 5,053 |
| TForceShellEffect | powers | 6 / 4,985 | 0 / 0 | 0 / 0 | 4,985 |
| Rigid-body shape builders (RF_GUI_Controls_Ex (unit routines) by address) | physics | 0 / 0 | 2 / 4,570 | 1 / 238 | 4,808 |
| RF_DataIO (unit routines) | data | 1 / 823 | 25 / 3,951 | 0 / 0 | 4,774 |
| SG_Thaum_DIsp (unit routines) | powers | 12 / 4,042 | 3 / 691 | 0 / 0 | 4,733 |
| RF_Main (unit routines) | runtime | 14 / 4,263 | 4 / 405 | 0 / 0 | 4,668 |
| FLX_ItemFactory (unit routines) | items | 4 / 4,535 | 0 / 0 | 0 / 0 | 4,535 |
| TTerrainManager | world | 6 / 3,351 | 6 / 1,033 | 0 / 0 | 4,384 |
| SG_Thaum_Force (unit routines) | powers | 8 / 3,829 | 3 / 289 | 1 / 181 | 4,299 |
| SG_CharacterAI (unit routines) | ai | 12 / 4,255 | 0 / 0 | 0 / 0 | 4,255 |
| TForceWaveEffect | powers | 4 / 4,246 | 0 / 0 | 0 / 0 | 4,246 |
| TTerrainGrass | world | 4 / 4,208 | 0 / 0 | 0 / 0 | 4,208 |
| RF_GUI_Controls_Ex (unit routines) | ui | 11 / 4,129 | 0 / 0 | 0 / 0 | 4,129 |
| Texture compression formats (DXT/LATC) | render | 8 / 3,532 | 2 / 43 | 1 / 473 | 4,048 |
| RF_Terrain_Materials (unit routines) | world | 2 / 2,994 | 5 / 1,029 | 0 / 0 | 4,023 |
| TLightSource | render | 8 / 3,486 | 1 / 494 | 0 / 0 | 3,980 |
| TFireEffectRow | powers | 4 / 3,971 | 0 / 0 | 0 / 0 | 3,971 |
| TDivCardForm | modes | 4 / 3,878 | 1 / 80 | 0 / 0 | 3,958 |
| RF_Materials (unit routines) | render | 2 / 3,866 | 0 / 0 | 0 / 0 | 3,866 |
| RFT_BoundSphere (unit routines) | core | 2 / 3,851 | 0 / 0 | 0 / 0 | 3,851 |
| TShellEffect | powers | 5 / 2,730 | 0 / 0 | 1 / 1,070 | 3,800 |
| FLX_Character_Motions (unit routines) | characters | 1 / 1,612 | 8 / 2,034 | 0 / 0 | 3,646 |
| TPwrForceKinesis_S | powers | 5 / 3,452 | 0 / 0 | 0 / 0 | 3,452 |
| RF_Terrain (unit routines) | world | 1 / 609 | 2 / 2,801 | 0 / 0 | 3,410 |
| Default material shader factory | render | 1 / 3,188 | 1 / 134 | 0 / 0 | 3,322 |
| TShadowCubeMap | render | 8 / 3,310 | 0 / 0 | 0 / 0 | 3,310 |
| RF_DBObjects (unit routines) | data | 4 / 2,779 | 3 / 524 | 0 / 0 | 3,303 |
| TShadowMapDirect | render | 4 / 3,259 | 0 / 0 | 0 / 0 | 3,259 |
| TGeometry | render | 6 / 2,254 | 0 / 0 | 1 / 804 | 3,058 |
| TItemGUIManager | items | 8 / 1,889 | 8 / 1,076 | 0 / 0 | 2,965 |
| XA_ExperienceSys (unit routines) | modes | 2 / 1,851 | 6 / 1,084 | 0 / 0 | 2,935 |
| TCharCreation | modes | 6 / 2,108 | 5 / 818 | 0 / 0 | 2,926 |
| RF_Polyhedra (unit routines) | render | 2 / 2,831 | 0 / 0 | 0 / 0 | 2,831 |
| .rfi image codec | render | 0 / 0 | 10 / 2,793 | 0 / 0 | 2,793 |
| TScanEffect | powers | 4 / 1,722 | 0 / 0 | 1 / 1,070 | 2,792 |
| TSelectionUI | ui | 9 / 2,745 | 0 / 0 | 0 / 0 | 2,745 |
| T3DNode | world | 7 / 1,202 | 3 / 1,190 | 1 / 324 | 2,716 |
| Pixel-format converters (8-bit and float) | render | 0 / 0 | 11 / 2,594 | 0 / 0 | 2,594 |
| TInventory | items | 9 / 2,417 | 1 / 41 | 1 / 69 | 2,527 |
| FLX_IconRender (unit routines) | render | 0 / 0 | 7 / 2,195 | 2 / 286 | 2,481 |
| TProjectManager | data | 4 / 2,454 | 0 / 0 | 0 / 0 | 2,454 |
| RF_Dynamics_Game (unit routines) | world | 7 / 1,969 | 3 / 470 | 0 / 0 | 2,439 |
| RFE_MeshTools (unit routines) | render | 1 / 37 | 6 / 2,341 | 0 / 0 | 2,378 |
| RF_kdTrees (unit routines) | world | 10 / 2,369 | 0 / 0 | 0 / 0 | 2,369 |
| TAudioEnvironment | audio | 3 / 2,097 | 2 / 270 | 0 / 0 | 2,367 |
| Item variant & condition randomisers (FLX_Character_Textures (unit routines) by address) | items | 2 / 2,319 | 0 / 0 | 0 / 0 | 2,319 |
| TThaumTree | powers | 7 / 2,045 | 3 / 246 | 0 / 0 | 2,291 |
| TArsenalView | items | 1 / 1,076 | 7 / 1,210 | 0 / 0 | 2,286 |
| RF_Dynamics_Animation (unit routines) | characters | 5 / 2,100 | 0 / 0 | 2 / 173 | 2,273 |
| TGridEnvManager | world | 3 / 2,199 | 0 / 0 | 0 / 0 | 2,199 |
| RF_Import_RFC2 (unit routines) | data | 7 / 1,515 | 2 / 600 | 0 / 0 | 2,115 |
| TScene | world | 1 / 740 | 8 / 1,355 | 0 / 0 | 2,095 |
| SG_GameFunctions (unit routines) | script | 6 / 1,405 | 3 / 673 | 1 / 11 | 2,089 |
| Equipment effects and focus-item slot (RFE_Effects_Electricity (unit routines) by address) | characters | 6 / 1,501 | 1 / 167 | 4 / 400 | 2,068 |
| TTextRenderer | ui | 1 / 2,043 | 0 / 0 | 0 / 0 | 2,043 |
| TForceBarrierEffect | powers | 2 / 2,028 | 0 / 0 | 0 / 0 | 2,028 |
| TPwrForceStrikes_C | powers | 4 / 1,977 | 1 / 43 | 0 / 0 | 2,020 |
| TMainMenu | ui | 7 / 1,707 | 2 / 305 | 0 / 0 | 2,012 |
| TClassSelect | modes | 5 / 1,940 | 0 / 0 | 0 / 0 | 1,940 |
| Shading-mask GLSL snippets | render | 2 / 1,895 | 1 / 38 | 0 / 0 | 1,933 |
| Scene content helpers (unit before RF_Dynamics) | world | 1 / 925 | 4 / 768 | 1 / 239 | 1,932 |
| TBubbleEffect | powers | 2 / 1,924 | 0 / 0 | 0 / 0 | 1,924 |
| RF_MeshVoxels (unit routines) | render | 1 / 1,434 | 1 / 468 | 0 / 0 | 1,902 |
| TkdTree | world | 3 / 1,661 | 1 / 240 | 0 / 0 | 1,901 |
| TSceneEnvironment | world | 4 / 1,897 | 0 / 0 | 0 / 0 | 1,897 |
| TLightAlias | render | 3 / 1,348 | 2 / 537 | 0 / 0 | 1,885 |
| SG_Dynamics_Effects (unit routines) | powers | 3 / 1,876 | 0 / 0 | 0 / 0 | 1,876 |
| TGUITask | ui | 8 / 1,372 | 5 / 492 | 0 / 0 | 1,864 |
| TPwrDispLongDoor_I | powers | 3 / 1,841 | 0 / 0 | 0 / 0 | 1,841 |
| TCharDecomp2 | characters | 5 / 1,766 | 0 / 0 | 0 / 0 | 1,766 |
| TRTDialogue | script | 2 / 1,734 | 0 / 0 | 0 / 0 | 1,734 |
| RF_Env_Effects (unit routines) | powers | 1 / 1,666 | 0 / 0 | 0 / 0 | 1,666 |
| XA_Arena save and rank helpers (TMatchBoard by address) | modes | 0 / 0 | 4 / 1,659 | 0 / 0 | 1,659 |
| FLX_PostProcess (unit routines) | render | 3 / 1,480 | 2 / 52 | 1 / 127 | 1,659 |
| TTransitionScreen | ui | 2 / 1,577 | 1 / 28 | 0 / 0 | 1,605 |
| SG_Thaumaturgy (unit routines) | powers | 4 / 1,296 | 1 / 278 | 0 / 0 | 1,574 |
| TBarricadeEntity | powers | 2 / 1,556 | 0 / 0 | 0 / 0 | 1,556 |
| TTErrainkdTree | world | 0 / 0 | 5 / 1,513 | 0 / 0 | 1,513 |
| RF_ObjectLibs (unit routines) | data | 4 / 1,227 | 1 / 284 | 0 / 0 | 1,511 |
| TWeaponBlood | combat | 2 / 1,494 | 0 / 0 | 0 / 0 | 1,494 |
| TPwrForceThrow_I | powers | 3 / 1,309 | 1 / 170 | 0 / 0 | 1,479 |
| Screen-quad GLSL snippets | render | 5 / 1,457 | 0 / 0 | 0 / 0 | 1,457 |
| TGridEnvBuilder | world | 2 / 431 | 4 / 1,004 | 0 / 0 | 1,435 |
| SG_EmblemGen (unit routines) | modes | 1 / 174 | 14 / 1,256 | 0 / 0 | 1,430 |
| TTerrainMesh | render | 3 / 1,408 | 0 / 0 | 0 / 0 | 1,408 |
| TGUIDrawSkinTex | ui | 4 / 1,400 | 0 / 0 | 0 / 0 | 1,400 |
| RF_UIDManager (unit routines) | data | 1 / 248 | 7 / 1,148 | 0 / 0 | 1,396 |
| SG_Teleporters (unit routines) | world | 3 / 1,390 | 0 / 0 | 0 / 0 | 1,390 |
| RF_Material_Manager (unit routines) | render | 2 / 1,201 | 1 / 180 | 0 / 0 | 1,381 |
| TMemBlockLibrary | data | 1 / 272 | 4 / 1,092 | 0 / 0 | 1,364 |
| RF_SceneEditor (unit routines) | world | 0 / 0 | 4 / 1,351 | 0 / 0 | 1,351 |
| FG_AIFunctions (unit routines) | script | 11 / 1,343 | 0 / 0 | 0 / 0 | 1,343 |
| TTexture | render | 2 / 872 | 2 / 451 | 0 / 0 | 1,323 |
| RF_Renderer (unit routines) | render | 0 / 0 | 6 / 1,315 | 0 / 0 | 1,315 |
| RF_GUI_Text_Edit (unit routines) | ui | 1 / 695 | 2 / 615 | 0 / 0 | 1,310 |
| RFE_MeshSubdivide (unit routines) | render | 4 / 1,186 | 0 / 0 | 1 / 96 | 1,282 |
| TForceDomeEffect | powers | 1 / 1,253 | 0 / 0 | 0 / 0 | 1,253 |
| TPostProcessor | render | 4 / 1,210 | 0 / 0 | 0 / 0 | 1,210 |
| TPwrDispVanish_I | powers | 2 / 1,175 | 0 / 0 | 0 / 0 | 1,175 |
| THairColPicker | ui | 2 / 1,159 | 0 / 0 | 0 / 0 | 1,159 |
| RFG_Dialogue (unit routines) | script | 1 / 17 | 5 / 1,130 | 0 / 0 | 1,147 |
| RF_GUI_Icons (unit routines) | ui | 1 / 652 | 2 / 481 | 0 / 0 | 1,133 |
| TSceneTaskInt | world | 0 / 0 | 4 / 1,133 | 0 / 0 | 1,133 |
| TBlastEntity | powers | 2 / 1,124 | 0 / 0 | 0 / 0 | 1,124 |
| Golem guardians and alarm zones (RFE_Effects_Electricity (unit routines) by address) | characters | 3 / 1,111 | 0 / 0 | 0 / 0 | 1,111 |
| TSector | world | 2 / 326 | 1 / 776 | 0 / 0 | 1,102 |
| SG_TradeSystem (unit routines) | modes | 4 / 1,029 | 1 / 69 | 0 / 0 | 1,098 |
| GUI support routines (unit before RF_GUI_Main) | ui | 0 / 0 | 2 / 1,089 | 0 / 0 | 1,089 |
| RFE_Effects_Electricity (unit routines) | powers | 2 / 1,050 | 0 / 0 | 0 / 0 | 1,050 |
| T3DView | render | 2 / 977 | 1 / 69 | 0 / 0 | 1,046 |
| TKFCCatRom3f | characters | 1 / 955 | 0 / 0 | 0 / 0 | 955 |
| THairEditForm | ui | 1 / 828 | 1 / 117 | 0 / 0 | 945 |
| RF_ShadowMap (unit routines) | render | 2 / 771 | 1 / 127 | 0 / 0 | 898 |
| TForceBlastGeometry | powers | 2 / 890 | 0 / 0 | 0 / 0 | 890 |
| TTendrilGeometry | powers | 2 / 890 | 0 / 0 | 0 / 0 | 890 |
| FLX_EffectRender (unit routines) | powers | 1 / 887 | 0 / 0 | 0 / 0 | 887 |
| TPwrEnergyShock_I | powers | 2 / 829 | 0 / 0 | 0 / 0 | 829 |
| TPwrSpiritForm | powers | 3 / 813 | 0 / 0 | 0 / 0 | 813 |
| OpenAL loader | audio | 1 / 781 | 0 / 0 | 0 / 0 | 781 |
| TCopperSatin | items | 1 / 776 | 0 / 0 | 0 / 0 | 776 |
| RF_GUI_Taskbar (unit routines) | ui | 4 / 574 | 1 / 198 | 0 / 0 | 772 |
| TRendererShaderFactory | render | 1 / 684 | 1 / 85 | 0 / 0 | 769 |
| TGemstone | items | 1 / 760 | 0 / 0 | 0 / 0 | 760 |
| TResin | items | 1 / 752 | 0 / 0 | 0 / 0 | 752 |
| TPwrDispSummonDemon_I | powers | 1 / 752 | 0 / 0 | 0 / 0 | 752 |
| TSteelPainted | items | 1 / 748 | 0 / 0 | 0 / 0 | 748 |
| TCopperAlloy | items | 1 / 748 | 0 / 0 | 0 / 0 | 748 |
| XA_HelpSystem (unit routines) | ui | 2 / 123 | 1 / 619 | 0 / 0 | 742 |
| TSteelChain | items | 1 / 732 | 0 / 0 | 0 / 0 | 732 |
| TToolTip | ui | 2 / 730 | 0 / 0 | 0 / 0 | 730 |
| TCacheShaderScreen | ui | 4 / 715 | 0 / 0 | 0 / 0 | 715 |
| TScriptRole | script | 1 / 712 | 0 / 0 | 0 / 0 | 712 |
| RFE_Effects_Blood (unit routines) | combat | 3 / 707 | 0 / 0 | 0 / 0 | 707 |
| TSteelGen | items | 1 / 700 | 0 / 0 | 0 / 0 | 700 |
| TIronGen | items | 1 / 700 | 0 / 0 | 0 / 0 | 700 |
| FPC System RTL | runtime | 6 / 675 | 1 / 23 | 0 / 0 | 698 |
| TSteelBlade | items | 1 / 692 | 0 / 0 | 0 / 0 | 692 |
| TSteelPlate2 | items | 1 / 692 | 0 / 0 | 0 / 0 | 692 |
| TEndSessionMenu | modes | 2 / 666 | 0 / 0 | 0 / 0 | 666 |
| TPickerForm | ui | 1 / 184 | 2 / 473 | 0 / 0 | 657 |
| TPwrControlMind_C | powers | 2 / 656 | 0 / 0 | 0 / 0 | 656 |
| TForceTorrentGeometry | powers | 1 / 651 | 0 / 0 | 0 / 0 | 651 |
| TStrikesGeometry | powers | 1 / 646 | 0 / 0 | 0 / 0 | 646 |
| Program main (Exanima.lpr) | runtime | 4 / 561 | 0 / 0 | 1 / 74 | 635 |
| RF_Splines (unit routines) | render | 1 / 629 | 0 / 0 | 0 / 0 | 629 |
| TStringLibrary | data | 0 / 0 | 4 / 615 | 0 / 0 | 615 |
| TForceWaveEntity | powers | 1 / 603 | 0 / 0 | 0 / 0 | 603 |
| Core string/number formatting utilities | core | 3 / 508 | 1 / 87 | 0 / 0 | 595 |
| TBarricadeGeometry | powers | 2 / 592 | 0 / 0 | 0 / 0 | 592 |
| TMaterialManager3 | render | 0 / 0 | 2 / 585 | 0 / 0 | 585 |
| TRBRotator | world | 3 / 583 | 0 / 0 | 0 / 0 | 583 |
| SG_Prelude_Main (unit routines) | modes | 2 / 575 | 0 / 0 | 0 / 0 | 575 |
| Colour helpers and parallel-for dispatch (RF_kdTrees (unit routines) by address) | core | 2 / 563 | 0 / 0 | 0 / 0 | 563 |
| TPlanksPainted | items | 1 / 548 | 0 / 0 | 0 / 0 | 548 |
| Scene node despawn and viewport step (TSceneTaskInt by address) | world | 0 / 0 | 2 / 544 | 0 / 0 | 544 |
| TPackageLibrary | data | 1 / 542 | 0 / 0 | 0 / 0 | 542 |
| TClothSilk | items | 1 / 536 | 0 / 0 | 0 / 0 | 536 |
| TClothPadded1 | items | 1 / 536 | 0 / 0 | 0 / 0 | 536 |
| TClothPadded2 | items | 1 / 536 | 0 / 0 | 0 / 0 | 536 |
| TCompanyCreate | modes | 0 / 0 | 2 / 529 | 0 / 0 | 529 |
| TClothVelvet | items | 1 / 528 | 0 / 0 | 0 / 0 | 528 |
| TMaterial | render | 2 / 285 | 1 / 232 | 0 / 0 | 517 |
| TClothBasic | items | 1 / 516 | 0 / 0 | 0 / 0 | 516 |
| TClothFine | items | 1 / 508 | 0 / 0 | 0 / 0 | 508 |
| RF_Environment (unit routines) | world | 0 / 0 | 4 / 497 | 0 / 0 | 497 |
| TWoodFineA | items | 1 / 488 | 0 / 0 | 0 / 0 | 488 |
| TWoodRoughA | items | 1 / 488 | 0 / 0 | 0 / 0 | 488 |
| FLX_GUI_Skinned (unit routines) | ui | 3 / 485 | 0 / 0 | 0 / 0 | 485 |
| TPwrLoyalty_E | powers | 1 / 360 | 1 / 124 | 0 / 0 | 484 |
| TTaskInterface | ui | 0 / 0 | 3 / 476 | 0 / 0 | 476 |
| FLX_GUI_Graphics (unit routines) | ui | 5 / 470 | 0 / 0 | 0 / 0 | 470 |
| RF_Terrain_Static (unit routines) | world | 0 / 0 | 1 / 468 | 0 / 0 | 468 |
| TDropDownList | ui | 2 / 210 | 1 / 256 | 0 / 0 | 466 |
| TCharViewBehaviour | ai | 1 / 466 | 0 / 0 | 0 / 0 | 466 |
| TLeatherA01 | items | 1 / 464 | 0 / 0 | 0 / 0 | 464 |
| TLeatherA02 | items | 1 / 464 | 0 / 0 | 0 / 0 | 464 |
| TPhysiquePicker | ui | 4 / 464 | 0 / 0 | 0 / 0 | 464 |
| TSuedeA01 | items | 1 / 456 | 0 / 0 | 0 / 0 | 456 |
| TWiredGripA01 | items | 1 / 456 | 0 / 0 | 0 / 0 | 456 |
| THelpScreen | ui | 2 / 177 | 2 / 199 | 2 / 77 | 453 |
| TShadowCubeTex | render | 1 / 237 | 1 / 206 | 0 / 0 | 443 |
| SGD_GameObjEdit (unit routines) | ui | 1 / 440 | 0 / 0 | 0 / 0 | 440 |
| FLXG_Character_Races (unit routines) | characters | 3 / 434 | 0 / 0 | 0 / 0 | 434 |
| TObjectEntity | ai | 0 / 0 | 2 / 431 | 0 / 0 | 431 |
| TReadKeyScreen | ui | 3 / 425 | 0 / 0 | 0 / 0 | 425 |
| TTextureManager | render | 2 / 417 | 0 / 0 | 0 / 0 | 417 |
| TPopUpDesc | items | 3 / 401 | 0 / 0 | 0 / 0 | 401 |
| TDataLibrary | data | 0 / 0 | 2 / 400 | 0 / 0 | 400 |
| RF_GUI_3DViewport (unit routines) | ui | 1 / 14 | 2 / 340 | 0 / 0 | 354 |
| TSubObject | render | 2 / 338 | 0 / 0 | 0 / 0 | 338 |
| TQuadFBO | render | 2 / 328 | 0 / 0 | 0 / 0 | 328 |
| Graphics & key settings (ini) | data | 1 / 126 | 1 / 201 | 0 / 0 | 327 |
| TObjLibManager | data | 1 / 321 | 0 / 0 | 0 / 0 | 321 |
| TCharLoot | items | 1 / 311 | 0 / 0 | 0 / 0 | 311 |
| Fire particle presets | powers | 2 / 309 | 0 / 0 | 0 / 0 | 309 |
| FLX_FX_Fire (unit routines) | powers | 1 / 300 | 0 / 0 | 0 / 0 | 300 |
| RF_DataManager (unit routines) | data | 1 / 295 | 0 / 0 | 0 / 0 | 295 |
| RF_GLGeometry (unit routines) | render | 0 / 0 | 0 / 0 | 1 / 290 | 290 |
| THelpScreen2 | ui | 0 / 0 | 1 / 288 | 0 / 0 | 288 |
| TGUIColSwatch | ui | 0 / 0 | 1 / 272 | 0 / 0 | 272 |
| FLXG_Pathfinding (unit routines) | ai | 2 / 265 | 0 / 0 | 0 / 0 | 265 |
| SG_GUI_Skills (unit routines) | ui | 0 / 0 | 1 / 197 | 1 / 67 | 264 |
| SG_Character_DBs (unit routines) | data | 1 / 18 | 3 / 239 | 0 / 0 | 257 |
| TVanishEffect | powers | 2 / 248 | 0 / 0 | 0 / 0 | 248 |
| TVoxelLight | render | 2 / 245 | 0 / 0 | 0 / 0 | 245 |
| TPopUpContainer | items | 1 / 245 | 0 / 0 | 0 / 0 | 245 |
| TDBObjManager | data | 1 / 245 | 0 / 0 | 0 / 0 | 245 |
| TJournal | script | 3 / 239 | 0 / 0 | 0 / 0 | 239 |
| TGameTips | script | 1 / 229 | 0 / 0 | 0 / 0 | 229 |
| TPwrMindSense_C | powers | 3 / 170 | 2 / 53 | 0 / 0 | 223 |
| TPopupParchment | ui | 2 / 220 | 0 / 0 | 0 / 0 | 220 |
| TCamera | render | 1 / 212 | 0 / 0 | 0 / 0 | 212 |
| TAbstractPathFinder | ai | 0 / 0 | 11 / 210 | 0 / 0 | 210 |
| RFE_Dynamics_Cloth (unit routines) | physics | 2 / 210 | 0 / 0 | 0 / 0 | 210 |
| TTexLoaderThread | render | 1 / 209 | 0 / 0 | 0 / 0 | 209 |
| TRFCExporter | data | 1 / 208 | 0 / 0 | 0 / 0 | 208 |
| TMotileBehaviour | characters | 1 / 27 | 3 / 178 | 0 / 0 | 205 |
| TVoxeliser | render | 0 / 0 | 1 / 193 | 0 / 0 | 193 |
| SG_Thaum_Energy (unit routines) | powers | 1 / 187 | 0 / 0 | 0 / 0 | 187 |
| TStreamThread | data | 0 / 0 | 1 / 181 | 0 / 0 | 181 |
| TAbstractToolTip | ui | 0 / 0 | 2 / 175 | 0 / 0 | 175 |
| RF_KFControllers (unit routines) | characters | 0 / 0 | 1 / 159 | 0 / 0 | 159 |
| RFA_Main (unit routines) | audio | 1 / 147 | 0 / 0 | 0 / 0 | 147 |
| TPwrForceBarricade_C | powers | 1 / 131 | 0 / 0 | 0 / 0 | 131 |
| TActivePower | powers | 2 / 122 | 2 / 6 | 0 / 0 | 128 |
| TPwrLoyalty_C | powers | 1 / 39 | 2 / 87 | 0 / 0 | 126 |
| RFP_Mipmaps_Kaiser (unit routines) | render | 0 / 0 | 2 / 124 | 0 / 0 | 124 |
| TPwrCommandDead_C | powers | 0 / 0 | 2 / 108 | 0 / 0 | 108 |
| TPwrCommandDead_E | powers | 1 / 107 | 0 / 0 | 0 / 0 | 107 |
| TCharBaseForm | ui | 1 / 102 | 0 / 0 | 0 / 0 | 102 |
| RF_StoreSettings (unit routines) | data | 2 / 99 | 0 / 0 | 0 / 0 | 99 |
| TTerrainPlugin | world | 0 / 0 | 1 / 94 | 0 / 0 | 94 |
| TShopWindow | modes | 1 / 89 | 0 / 0 | 0 / 0 | 89 |
| TCharEditForm | ui | 1 / 84 | 0 / 0 | 0 / 0 | 84 |
| TMessageBox | ui | 0 / 0 | 1 / 83 | 0 / 0 | 83 |
| FLX_Entities (unit routines) | ai | 1 / 82 | 0 / 0 | 0 / 0 | 82 |
| TStatusForm | ui | 1 / 78 | 0 / 0 | 0 / 0 | 78 |
| TEntity | ai | 2 / 24 | 1 / 48 | 0 / 0 | 72 |
| TPhysicsController | physics | 2 / 51 | 2 / 15 | 0 / 0 | 66 |
| TResetToPortalDlg | ui | 0 / 0 | 1 / 65 | 0 / 0 | 65 |
| TConfirmDlg | ui | 0 / 0 | 1 / 65 | 0 / 0 | 65 |
| TConfirmDismissDlg | ui | 0 / 0 | 1 / 65 | 0 / 0 | 65 |
| TConfirmDialogue | ui | 0 / 0 | 1 / 65 | 0 / 0 | 65 |
| TScriptActor | script | 1 / 61 | 0 / 0 | 0 / 0 | 61 |
| TRBPushSurface | world | 1 / 53 | 0 / 0 | 0 / 0 | 53 |
| TPwrForceShell_C | powers | 0 / 0 | 1 / 43 | 0 / 0 | 43 |
| TArsenalContFrame | items | 0 / 0 | 1 / 40 | 0 / 0 | 40 |
| TKFCLinear3f | characters | 0 / 0 | 1 / 36 | 0 / 0 | 36 |
| Steam API glue | runtime | 0 / 0 | 1 / 12 | 0 / 0 | 12 |
