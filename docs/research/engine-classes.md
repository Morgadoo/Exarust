# Engine class map

`Exanima.exe` is a 64-bit Free Pascal 3.2.2 program (`FPC 3.2.2 [2023/03/04]
for x86_64 - Win64`). FPC keeps run-time type information, so the names of
~600 engine classes survive in the binary. Together they give a high-level
map of the original engine, which this project uses to decide what to
rebuild and where. Names are listed for orientation only: Exarust code uses
its own names (see `AGENT.md`).

Imports are only `kernel32`, `user32`, `gdi32`, `winmm`, `ole32/oleaut32`,
`shell32`, `comctl32`; OpenGL (`opengl32` via `wglGetProcAddress`), OpenAL
Soft (`OpenAL64.dll`) and Steam (`steam_api64.dll`) are loaded at runtime.

## Technology facts derived from the binary

| area | evidence | consequence for Exarust |
|---|---|---|
| Graphics API | `glTexStorage2D`, `glGetUniformBlockIndex`, `glDrawElementsInstanced`, `glFramebufferTextureLayer`, `wglCreateContextAttribsARB` | OpenGL 4.2+ core feature level — wgpu covers it |
| Texture compression | `GL_EXT_texture_compression_s3tc`, `…_rgtc`, `…_latc` | BC1/BC3/BC4/BC5 upload directly in wgpu (`TEXTURE_COMPRESSION_BC`) |
| Global illumination | `TVXGIManager`, `TVXGIMap`, `TVoxeliser`, `TVoxelLight`, `TGIMapBlock`, `TGlobalAOMap` | voxel GI + baked AO; Bevy has no VXGI → custom pass (Phase 5) |
| Shadows | `TShadowCubeMap`, `TShadowSpotTex`, `TShadowMapDirect` | point-light cube shadows, spot, directional |
| Shaders | `TShaderLibrary`, `TShaderBuilder`, `TGPUProgDictionary`, `.flb` libraries | GLSL generated from fragments; we author WGSL |
| Procedural textures | `TTextureProg`, `TCharTexGenerator`, `TItemTexGen`, `TProcMaterial`, `textures.flb` | GPU texture generation for characters/items |
| Audio | `alGenEffects`, `alAuxiliaryEffectSlot*`, `TAudioEnvironment`, `oalsoft.ini` | OpenAL Soft + EFX reverb |
| Physics | `TRBD*`, `TTetraRBD`, `TCollisionSolver`, `TClothFine/Padded/Silk/Velvet`, `TMotileDynamics` | custom rigid/articulated bodies + cloth |
| Threads | `TStreamThread`, `TTexLoaderThread`, `TKDtreeBuilderThread`, `TStaticTerrainThread`, `TInjectionThread` | async streaming of textures/terrain |
| Serialization | `TRFCReader`, `TRFCParser`, `TRFCExporter`, `TChunkStream`, `TDataChunkStream` | `.rfc` is a chunk stream (matches `exa_formats::chunk`) |

## Classes by subsystem

### Physics, bodies & cloth (34)

`TClothBasic`, `TClothConstraint`, `TClothConstraints`, `TClothFine`, `TClothLink`, `TClothPadded1`, `TClothPadded2`, `TClothSilk`, `TClothVelvet`, `TCollisionSolver`, `TConvexVol`, `TDynamicsEnv`, `TForceChunk`, `TGetRBDClass`, `THitRec`, `TMotileDynamics`, `TParCnstrnt`, `TPhaseHit`, `TPhysicsController`, `TPolyHedron`, `TRBBlncPt`, `TRBDArray`, `TRBDContacts`, `TRBDDoor`, `TRBDDoor8`, `TRBLink`, `TRBPushSurface`, `TRBRotator`, `TSimObject`, `TSimSector`, `TSimulation`, `TSimulation8`, `TTetraRBD`, `TVCCloth`

### Characters, motion & animation (46)

`TASeqNodeKF`, `TASeqPtr`, `TActiveAnim`, `TAnimNoise`, `TBaseGUISkin`, `TBaseGUISkinp`, `TCachedMotionSet`, `TCharDecomp2`, `TCharTexGenerator`, `TCombatant`, `TDASkinPivot`, `TGUIDrawSkin`, `TGUIDrawSkinTex`, `TGenderSpecifier`, `TGeomController`, `TKFCCatRom3f`, `TKFCLinear3f`, `TKFController`, `TLinear3fKF`, `TMCSeqData`, `TMCSequenceHeader`, `TMCSequenceSet`, `TMorphController`, `TMorphPoint`, `TMorphTarget`, `TMotileAI`, `TMotileBehaviour`, `TMotileBehaviourH`, `TMotilePart`, `TMotileRace`, `TMotileRacep`, `TMotionController`, `TPartTransform`, `TPhysiquePicker8`, `TRaceDBX`, `TSGCharacter`, `TSeqNodeKF`, `TSeqNodeKFx`, `TSkinBuilder`, `TSkinController2`, `TSkinController2P`, `TSkinEdge`, `TSkinGroup`, `TSkinPivot`, `TSkinningManager2`, `TVertexController`

### Combat, weapons & damage (13)

`TApparelEval`, `TBloodSprays`, `TBloodyWeapon`, `TCoverageSolution`, `TPwrShockWeapons`, `TWeaponBlood`, `TWeaponBloodp`, `TWeaponChoices`, `TWeaponEval`, `TWeaponGrip`, `TWeaponType`, `TWeaponTypeSet`, `TWiredGripA01`

### Items, materials & crafting (75)

`TADBDrpTblItem`, `TArsenalContFrame`, `TArsenalView`, `TCDropTable`, `TCDropTablep`, `TCharLoot`, `TCharLootHz1`, `TCharacterFactory`, `TCompVariant`, `TContainerItem`, `TCopperAlloy`, `TCopperSatin`, `TDAGameItemRef`, `TDBDrpTblItem`, `TFactoryMaterial`, `TFactoryMaterial2`, `TFactoryMaterial3`, `TGUIItemContainer`, `TGUIItemView`, `TGUITableItem`, `TGameItemRef`, `TGemstone`, `TGemstonep`, `TInventory`, `TIronGen`, `TItemAbility`, `TItemChance`, `TItemComponent`, `TItemFactory`, `TItemGUIManager`, `TItemGrade`, `TItemIDList`, `TItemLocation`, `TItemRenderer`, `TItemRendererX`, `TItemTexGen`, `TLeatherA01`, `TLeatherA02`, `TLeatherA028`, `TLightListItem`, `TLinkedListItem`, `TListItemArray`, `TLoadedItem`, `TMainMenuItem`, `TMenuItem`, `TMenuItems`, `TOprtvQItem`, `TPlanksPainted`, `TPopUpContainerx`, `TRTDropTable`, `TRandomItemPars`, `TRandomItemParsp`, `TRendererShaderFactory`, `TReportItem`, `TResin`, `TSGContainerItem`, `TSGItemContainer`, `TSceneMenuItem`, `TShaderFactory`, `TSpawnItem`, `TSteelBlade`, `TSteelChain`, `TSteelGen`, `TSteelPainted`, `TSteelPlate2`, `TSteelPlate2h`, `TSuedeA01`, `TSuedeA01P`, `TTexBindItem`, `TUpdatingItem`, `TUserItem`, `TWoodFineA`, `TWoodRoughA`, `TWoodRoughAX`, `TWorkQueueItem`

### AI & behaviour (12)

`TAIActivity`, `TAIDRng`, `TAISearchActivity`, `TAbstractPathFinder`, `TAnyPlayerBehave`, `TCharViewBehaviour`, `TDirectiveClass`, `TDirectiveWord`, `TIdleActivityReg`, `TPathFinder`, `TPlayerBehaviour`, `TSensedMind`

### Thaumaturgy (powers) & effects (96)

`TAbstractForceBarrier`, `TActivePower`, `TBarricadeEmissive`, `TBarricadeEmissive0`, `TBarricadeEntity`, `TBarricadeGeometry`, `TBarrierEmissive`, `TBlastEntity`, `TBlastEntityX`, `TBubbleEffect`, `TBubbleEmissive8`, `TBubbleGeometry`, `TCuboidEmissive`, `TDispDissolve`, `TDisplaceEffect`, `TDisplayMode`, `TEffectGeometry`, `TEmitter`, `TFireEffectRow`, `TForceBarricadeEffect`, `TForceBarrierEffect`, `TForceBarrierGeometry`, `TForceBlastEffect`, `TForceBlastEmissive`, `TForceBlastGeometry`, `TForceBubbleEffect`, `TForceBubbleEffectH`, `TForceDomeEffect`, `TForceGeometry`, `TForceShellEffect`, `TForceTendril`, `TForceTorrentEffect`, `TForceTorrentEmissive`, `TForceTorrentGeometry`, `TForceWaveEffect`, `TForceWaveEffectH`, `TForceWaveEntity`, `TForceWaveEntity8`, `TGIEmissive`, `TGIEmissivep`, `TMindControlEffect`, `TMindGlow`, `TMindGlowh`, `TParticle`, `TParticleSysDescriptor`, `TParticleSystem`, `TPortalCon`, `TPortalEffect`, `TPwrCnct`, `TPwrCommandDead`, `TPwrConfusion`, `TPwrConstantDisp`, `TPwrControlMind`, `TPwrDispLongDoor`, `TPwrDispSummonDemon`, `TPwrDispVanish`, `TPwrEnergyShock`, `TPwrEnrage`, `TPwrForceBarricade`, `TPwrForceBubble`, `TPwrForceKinesis`, `TPwrForceShell`, `TPwrForceStrikes`, `TPwrForceThrow`, `TPwrLoyalty`, `TPwrMindMask`, `TPwrMindSense`, `TPwrNode`, `TPwrPosesses`, `TPwrSpiritForm`, `TPwrSpiritForm8`, `TPwrSuspect`, `TPwrTrust`, `TRainEffect`, `TResetToPortalDlg`, `TResetToPortalDlghT1`, `TScanEffect`, `TShellEffect`, `TShellEffecth`, `TStrikesEmissive`, `TStrikesGeometry`, `TTEMindSense`, `TTESpiritRealm`, `TTESpiritRealmP`, `TTeleportEffectP`, `TTendril`, `TTendrilEffect`, `TTendrilEmissive`, `TTendrilEmissive0`, `TTendrilEmitter`, `TTendrilGeometry`, `TTendrilStream`, `TThaumTree`, `TTorrentStream`, `TVanishEffect`, `TVanishGeom`

### Rendering & GPU (94)

`TAGPUDictClass`, `TAGPUDictWordH`, `TAGrassVBE`, `TALight`, `TATrnMatLyr`, `TAbstractRenderer`, `TAbstractRenderer0`, `TAbstractShadowMap`, `TAsyncIconPars`, `TAsyncIconPars8`, `TAsyncIconRenderA`, `TAsyncIconRenderAH`, `TCacheShaderScreen`, `TCachedTextureLib`, `TCachedTextureLibp`, `TCamera`, `TDepthRB`, `TExPixelFormat`, `TFrameBuffer`, `TGIMapBlock`, `TGIMapBlock0F3`, `TGLGeometry`, `TGPUProgDictionary`, `TGPUProgram`, `TGSLines`, `TGeometry`, `TGeometryp`, `TGlobalAOMap`, `TGrassGeom`, `TGrassVBE`, `TIconManager`, `TIconRenderer`, `TLgtOccluder`, `TLightAlias`, `TLightSource`, `TMatImplementer`, `TMaterial`, `TMaterialManager3`, `TMaterialManager3P`, `TMaterialPlugIn`, `TMaterialRefList`, `TMeshModifier`, `TMeshSubdivider`, `TNavCameraControl`, `TOGLTexFormat`, `TParamRenderProc`, `TPostProcessor`, `TProcMaterial`, `TProcMaterialx`, `TQuadFBO`, `TQuadFBO0h0`, `TRFImage`, `TRTMatPlugIn`, `TRenderer`, `TResTexture`, `TSelectIcon`, `TShadeProgramSet`, `TShadeStateQuery`, `TShader`, `TShaderBuilder`, `TShaderLibrary`, `TShaderList`, `TShaderSet`, `TShadowCubeMap`, `TShadowCubeTex`, `TShadowCubeTexH40`, `TShadowMap`, `TShadowMapDirect`, `TShadowMapDirect020`, `TShadowSpotTex`, `TSplineRender`, `TSplineRenderX`, `TSurfaceCache`, `TTerrainGrass`, `TTerrainMesh`, `TTexLoaderThread`, `TTexLoaderThreadp`, `TTextRenderer`, `TTexture`, `TTextureManager`, `TTextureProg`, `TTextureSet`, `TTextureSet8`, `TTileTexture`, `TTrnTexProgManager`, `TVXGIManager`, `TVXGIManagerX`, `TVXGIMap`, `TVXGIMapp`, `TVertexData`, `TVoxelLight`, `TVoxelLightP`, `TVoxelMap`, `TVoxeliser`

### World, levels & terrain (43)

`TABlockBrushData`, `TAPTerrainMat`, `TATerrainTileh`, `TAudioEnvironment`, `TBlockBrushData`, `TBlockFoamMap`, `TBlockNrmMap`, `TCachedFloor`, `TDummyAudioEnv`, `TFloorRayCastFace`, `TGEFloorSet`, `TGEWallSet`, `TGetTerrainDetailImplemeter`, `TGridEnvBuilder`, `TGridEnvManager`, `TGridEnvRoom`, `TKDtreeBuilderThread`, `TMapDiscovery`, `TMemBlockLibrary`, `TPropagation`, `TScene`, `TSceneEnvironment`, `TScenePlugin`, `TSceneTaskInt`, `TSector`, `TSectorh`, `TStaticTerrain`, `TStaticTerrainHk2`, `TStaticTerrainThread`, `TTErrainkdTree`, `TTErrainkdTreeH`, `TTerrainBlock`, `TTerrainBrushMgr`, `TTerrainDetailImplementer`, `TTerrainDetailImplementerpL2`, `TTerrainDetails`, `TTerrainManager`, `TTerrainMat`, `TTerrainPlugin`, `TTerrainPluginpK2`, `TTerrainTile`, `TTerrainTilePW2`, `TTileMask`

### Audio (8)

`TCurUtter`, `TMoveSound`, `TSndBufEntry`, `TSoundSet`, `TSoundSource`, `TUtteranceChoice`, `TVirtualSoundSource`, `TVoiceSet`

### Script, dialogue & story (29)

`TConfirmDialogue`, `TConfirmDialogueP`, `TConfirmDismissDlg`, `TConfirmDismissDlgh`, `TConfirmDlg`, `TConfirmDlgPV1`, `TDescPhrase`, `TDialogueChoices`, `TDlgChoice`, `TDlgTopic`, `TGameTips`, `TJournal`, `TJournalEntry`, `TPhraseFormat`, `TPlayerExperiences`, `TPlayerTopic`, `TPopUpDesc`, `TProjectDesc`, `TRTDialogue`, `TSGSettingDlg`, `TScriptActor`, `TScriptClass`, `TScriptClassVar`, `TScriptFunction`, `TScriptInst`, `TScriptRole`, `TScriptText`, `TScriptVariable`, `TScriptWord`

### Game modes, arena & session (27)

`TArenaMatch`, `TCharCreation`, `TCharacterLoadout`, `TClassSelect`, `TCompanyCreate`, `TCompanyCreatep`, `TDBObjManager`, `TDivCardForm`, `TDomainWheel`, `TEndSessionMenu`, `TFightCompany`, `TGameMode`, `TGameSessionManager`, `TGameSessionManagerp41`, `THireling`, `TLargeDataManager`, `TLoadoutForm`, `TManageForm`, `TMatchBoard`, `TMatchBoard8`, `TObjLibManager`, `TProjectManager`, `TSaveGameManager`, `TSaveGameManagerP`, `TShopWindow`, `TShopWindowp22`, `TUIDManager`

### UI (50)

`TAbstractToolTip`, `TAbstractToolTiph`, `TCharBaseForm`, `TCharEditForm`, `TClipBoardFormat`, `TColourPicker`, `TColourPickerpo0`, `TCompactSkillUI`, `TCursor`, `TCursorAction`, `TDropDownList`, `TEmblemForm`, `TFileFormat`, `TFormClass`, `TGUIColSwatch`, `TGUIControl`, `TGUIControlClass`, `TGUIForm`, `TGUIMenu`, `TGUITableColumn`, `TGUITableView`, `TGUITableViewp`, `TGUITask`, `TGUITreeView`, `TGetTipClass`, `THairColPicker`, `THairEditForm`, `THelpScreen`, `THelpScreen2`, `THelpTip`, `TMainMenu`, `TMessageBox`, `TPickerForm`, `TPickerFormP`, `TPopupParchment`, `TReadKeyScreen`, `TSelectionUI`, `TSelectionUI0`, `TSkillSelector`, `TSkillTip`, `TSkillUI`, `TSkillUIDat`, `TSplashScreen`, `TStatusForm`, `TTaskMenu`, `TTextBox`, `TTonePicker`, `TToolTip`, `TToolTipX`, `TTransitionScreen`

### Data, assets & I/O (29)

`TAUIDEntryC`, `TCharacterDB`, `TCharacterDBhq2`, `TChunkStream`, `TDataChunkStream`, `TDataEntry`, `TDataLibrary`, `TDataStream`, `TExtNodeChunk`, `TFileCategory`, `TFilePath`, `TLocaleDB`, `TLocaleDBX`, `TLocaleData2`, `TObjectDB`, `TObjectDBP`, `TPackageLibraryH`, `TRFCExporter`, `TRFCExporterx`, `TRFCParser`, `TRFCReader`, `TRFTimer`, `TSharedFilePath`, `TSharedFilePaths`, `TSklTecData`, `TStreamThread`, `TStringLibrary`, `TUIDDataSetC`, `TUIDEntryC`

### Core, math & threading (38)

`TA2x2i`, `TA2x3i`, `TA2x4f`, `TA2xV4f`, `TA4x3f0`, `TAByte`, `TADword`, `TAFaceVRef`, `TAFloat`, `TAbstractObjectWriterh`, `TAnInt`, `TAnUInt`, `TAnx2i`, `TApplicationTask`, `TBuildListVisitor`, `TFilter1D`, `TFilterSet`, `TIDRng`, `TInitHandler`, `TInjectionThread`, `TInjectionThreadhA3`, `TLinkedList`, `TLinkedListVisitor`, `TMLThread`, `TMultiLoop`, `TRecti`, `TRemoveReferenceVisitor`, `TResolveReferenceVisitor`, `TSmallFloat`, `TTaskInterface`, `TThreadHe`, `TUpdtSupport`, `TVec2f`, `TVec2w`, `TVec3f`, `TVec3w`, `TVec4f`, `TWorkerThread`

### Other (37)

`TCStrConv`, `TCharCondition`, `TCharacterView`, `TCstmNavMode`, `TDA3DNode`, `TEdgSplt`, `TElementAlias`, `TElementAliasArray`, `TEntity`, `TEnumeratedAliases`, `TEnumeratedAliasesArray`, `TExportNode`, `TGameActQI`, `TGameActionMgr`, `TGameEditControls`, `THcSDHcK0H`, `TIniSetting`, `TKeyDef`, `TLocalUnResolvedReference`, `TObjectEntity`, `TObjecthQ`, `TPlayerControl`, `TPosComponent`, `TShadingState`, `TStackEntry`, `TStructElSet`, `TStructElement`, `TSubObject`, `TSubObjectArray`, `TTreeNode`, `TUTF7EncodingpS4`, `TUnResolvedInstance`, `TUnicodeEncoding0V4`, `TUnresolvedReference`, `TUserMessage`, `TUserMsg`, `TVertexMapping`

