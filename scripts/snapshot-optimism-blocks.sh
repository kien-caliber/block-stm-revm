#!/bin/sh

set -e

if [ ! -f Cargo.toml ]; then
    echo "Error: Cargo.toml not found in the current directory."
    exit 1
fi

for blk in 124830410 124828225 124824124 124755589 124684166 124647568 124637482 124609712 124525634 124506268 124480106 124415987 124365631 124355357 124353653 124281100 124227892 124223539 124149991 124132289 124072796 123970589 123926778 123858330 123824381 123809673 123806224 123797654 123762564 123762412 123720689 123685397 123507111 123499678 123485425 123482466 123477237 123476636 123476299 123445010 123404462 123382069 123283270 123279725 123240835 123235241 123139954 123130069 123086152 123058986 122968648 122915651 122847407 122825503 122810742 122793158 122781046 122716218 122691635 122680324 122656003 122648811 122648235 122628003 122611801 122600466 122591132 122568179 122566627 122517178 122466134 122365220 122322291 122312393 122292482 122260017 122176546 121994255 121986257 121979325 121881362 121814720 121712608 121700504 121689343 121653356 121527569 121492230 121481583 121457626 121445648 121411596 121210038 121204589 121162987 121110493 121030761 120922875 120880795 120877995 120791126 120713474 120708004 120705120 120642699 120549977 120529667 120503295 120491575 120463371 120462641 120436794 120432134 120396105 120365436 120337193 120291188 120277606 120259176 120235471 120213631 120197745 120172810 120118072 120116966 120081794 120036599 120011577 119942924 119878666 119744938 119734455 119643245 119642073 119584838 119503377 119466251 119411756 119394080 119322804 119188144 119142487 119101989 119099642 119089336 119067387 119016966 118989435 118914113 118836540 118802449 118797328 118621092 118511870 118506119 118486950 118389214 118372836 118330601 118224351 118176725 118138291 118129993 118033674 118004430 117995179 117989240 117938107 117936449 117927700 117849687 117834834 117833916 117776676 117752009 117703283 117699942 117685682 117678535 117477958 117394061 117389776 117359325 117334503 117316076 117297377 117258845 117190901 117187993 117110364 117107889 117096820 117085158 116971101 116951449 116950409 116859469 116845096 116840668 116817158 116779567 116732135 116672724 116650304 116585974 116475749 116420422 116419022 116268240 116266576 116162905 116106323 116045238 116041976 116013972 115998858 115996091 115978639 115973975 115966959 115910082 115888384 115883017 115836293 115719499 115624666 115584870 115583794 115575634 115543890 115444314 115442162 115432865 115429600 115402639 115331131 115280543 115246713 115213427 115166288 115129939 115122767 115113215 115002685 114951014 114939717 114934597 114915056 114885904 114864841 114823805 114819892 114802753 114791380 114740691 114740500 114727835 114643830 114609965 114582114 114566362 114553014 114503057 114502919 114487145 114465873 114363665 114356874 114315335 114297327 114284157 114277089 114230012 114181655 114168381 114153231 114106822 113959344 113930658 113774050 113766057 113729116 113554633 113512155 113506445 113379406 113222509 113143467 113138724 113089303 113074562 113048833 112971458 112959284 112931246 112849790 112757754 112639335 112631136 112549785 112467028 112454678 112428489 112370281 112359664 112296020 112270334 112178105 112167594 112162938 112140031 112101123 112048629 112013252 112004197 111966777 111840914 111840009 111797849 111783285 111696785 111683273 111644313 111575302 111565618 111549003 111494814 111457065 111452279 111403463 111351816 111276586 111110393 111109653 111059296 110938763 110825740 110801234 110797081 110783477 110706500 110704311 110693604 110669833 110633156 110534097 110522804 110444907 110444597 110429830 110409794 110389284 110381593 110360922 110280877 110277850 110277790 110256463 110220197 110180422 110166609 110140773 110126791 110095355 110051188 110014493 110003201 109957776 109908398 109908141 109795429 109772268 109753415 109729811 109705574 109699530 109692324 109655620 109654744 109612692 109594430 109583142 109476929 109442609 109431246 109429011 109404080 109403867 109345325 109259995 109184635 109125573 108906659 108873545 108864082 108850145 108840509 108817621 108778943 108777002 108772992 108732641 108654656 108587430 108573381 108560347 108526119 108505522 108484626 108479348 108433674 108431075 108393985 108382581 108381796 108365186 108307605 108245868 108235596 108210604 108188230 108185650 108150842 108127149 108083753 108045762 108035574 107993000 107965607 107921332 107911043 107886856 107856460 107845030 107736952 107717256 107612030 107597347 107576476 107530043 107505710 107418252 107409176 107392775 107389890 107348821 107306589 107195732 107176524 107168863 107099162 107088729 107037121 107026068 107018288 107001071 106938349 106937143 106879742 106843994 106830444 106811301 106779904 106729463 106614763 106608775 106572728 106568161 106462744 106381313 106350587 106347978 106297576 106290433 106266610 106219036 106144838 106109687 106002081 105955827 105931042 105898060 105845123 105791071 105768286 105751816 105697201 105678695 105668486 105663017 105572090 105469673 105436041 105375714 105360580 105354349 105312076 105272914 105255601 105241123; do
    echo $blk
    cargo run --example snapshot-optimism-block $blk \
        --write-block-to target/benches/optimism/$blk/block.json \
        --write-pre-state-to target/benches/optimism/$blk/pre_state.json \
        --write-bytecodes-to target/benches/optimism/bytecodes.bincode
done
